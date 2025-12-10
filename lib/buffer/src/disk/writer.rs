use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::marker::PhantomData;
use std::sync::Arc;

use bytes::BufMut;
use tracing::{debug, error, trace, warn};

use super::record::validate_last_write;
use super::{Config, Ledger};
use crate::Encodable;

/// Using 256KB as it aligns nicely with the I/O size exposed by major cloud providers.
/// This may not be the underlying block size used by the OS, but it still aligns well
/// with what will happen on the `backend` for cloud providers, which is simply a useful
/// default for when we want to look at buffer throughput and estimate how many IOPS
/// will be consumed, etc
const DEFAULT_WRITE_BUFFER_SIZE: usize = 256 * 1024; // 256KB

const FORCE_FLUSH_BYTES: usize = 8 * 1024 * 1024;

/// Error that occurred during calls to [`Writer`]
#[derive(Debug, thiserror::Error)]
pub enum Error<T: Encodable> {
    /// A general I/O error occurred.
    ///
    /// Different methods will capture specific I/O errors depending on the situation, as some
    /// errors may be expected and considered normal by design. For all I/O errors that are
    /// considered atypical, they will be returned as this variant.
    #[error("write I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The record attempting to be written was too large.
    ///
    /// In practice, most encoders will throw their own error if they cannot write all
    /// of the necessary bytes during encoding, and so this error will typically only
    /// be emitted when the encoder throws no error during the encoding step itself, but
    /// manages to fill up the encoding buffer to the limit.
    #[error("record too large: limit is {limit}")]
    RecordTooLarge { limit: usize, size: usize },

    /// The chunk file did not have enough remaining space to write the record.
    ///
    /// This could be because the chunk file is legitimately full, but is more commonly
    /// related to a record being big enough that it would exceed the max chunk file size.
    ///
    /// The record that was given to write is returned.
    #[error("chunk file full or record would exceed max_chunk_size")]
    FileFull { record: T, serialized_len: usize },

    /// The writer failed to validate the last written record
    #[error("validate record failed")]
    Validate(String),

    /// The encoder encountered an issue during encoding
    ///
    /// For common encoders, failure to write all the bytes of the input will be the
    /// most common error, and in fact, some encoders, it's the only possible error that
    /// can occur.
    #[error("failed to encode record: {0:?}")]
    Encode(<T as Encodable>::Error),

    #[error("writer is closed")]
    Closed,
}

pub struct Writer<T> {
    config: Config,
    ledger: Arc<Ledger>,

    buf: Vec<u8>,
    inner: File,

    // The written bytes of current file
    write_size: usize,
    next_record_id: u64,

    // everytime flush called, unflushed_* should be reset too
    unflushed_bytes: usize,
    unflushed_records: usize,

    _pd: PhantomData<T>,
}

impl<T> Drop for Writer<T> {
    fn drop(&mut self) {
        debug!(message = "writer marked as closed");

        // close the writer, and signals the readers that no more records shall be read
        if self.ledger.mark_done() {
            self.ledger.notify_readers();
        }
    }
}

impl<T: Encodable> Writer<T> {
    pub fn new(config: Config, ledger: Arc<Ledger>) -> std::io::Result<Self> {
        loop {
            let id = ledger.get_next_write_record_id().wrapping_sub(1);
            let writer_id = ledger.get_current_writer_file_id();
            let path = config.chunk_path(writer_id);

            if !path.exists() {
                // create the chunk file is it is not exists
                OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&path)?;
            }

            if let Err(err) = validate_last_write(&path, id) {
                // writer leave the bad/corrupt file, then reader will skip and delete it
                warn!(
                    message = "validate chunk file failed, roll to next file",
                    ?path,
                    ?err
                );

                ledger.increase_writer_file_id();
            } else {
                break;
            }
        }

        let writer_file_id = ledger.get_current_writer_file_id();
        let chunk_path = config.chunk_path(writer_file_id);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(chunk_path)?;

        let next_record_id = ledger.get_next_write_record_id();
        let write_size = file.metadata()?.len() as usize;

        Ok(Writer {
            config,
            ledger,
            buf: Vec::with_capacity(DEFAULT_WRITE_BUFFER_SIZE),
            inner: file,
            next_record_id,
            write_size,
            unflushed_bytes: 0,
            unflushed_records: 0,
            _pd: PhantomData,
        })
    }

    pub async fn try_write(&mut self, record: T) -> Result<Option<T>, Error<T>> {
        let required = self.encode(&record)?;

        if required > self.config.max_record_size {
            return Err(Error::RecordTooLarge {
                limit: self.config.max_record_size,
                size: required,
            });
        }

        match self.try_write_inner(record, required).await? {
            Ok(_written) => Ok(None),
            Err(item) => Ok(Some(item)),
        }
    }

    pub async fn write(&mut self, mut record: T) -> Result<usize, Error<T>> {
        let required = self.encode(&record)?;
        if required > self.config.max_record_size {
            return Err(Error::RecordTooLarge {
                limit: self.config.max_record_size,
                size: required,
            });
        }

        loop {
            match self.try_write_inner(record, required).await? {
                Ok(written) => return Ok(written),
                Err(item) => {
                    record = item;
                    self.ledger.wait_for_write().await;
                }
            }
        }
    }

    async fn try_write_inner(
        &mut self,
        record: T,
        required: usize,
    ) -> Result<Result<usize, T>, Error<T>> {
        loop {
            if self.ledger.done() {
                return Err(Error::Closed);
            }

            if self.ledger.get_buffer_bytes() + required > self.config.max_buffer_size {
                return Ok(Err(record));
            }

            // chunk file should not overflow `max_chunk_size`
            if self.write_size + required <= self.config.max_chunk_size {
                self.inner.write_all(&self.buf)?;

                self.ledger.notify_readers();
                self.ledger.track_write(required);

                trace!(
                    message = "wrote record success",
                    id = self.next_record_id,
                    size = required,
                    chunk = self.ledger.get_current_writer_file_id(),
                    chunk_size = self.write_size + required,
                );

                break;
            }

            self.roll_to_next_writer().await?;

            // NB no need to notify other writers, cause the writer is protected by Mutex,
            // so there is no other writer
        }

        self.write_size += required;
        self.unflushed_bytes += required;
        self.unflushed_records += 1;
        self.next_record_id = self.next_record_id.wrapping_add(1);

        // force flush if there is too much data unflushed
        if self.unflushed_bytes >= FORCE_FLUSH_BYTES {
            self.flush()?;
        }

        Ok(Ok(required))
    }

    fn encode(&mut self, record: &T) -> Result<usize, Error<T>> {
        self.buf.clear();

        // set length and crc32 placeholder
        self.buf
            .extend_from_slice(&[0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0]);
        self.buf
            .extend_from_slice(&self.next_record_id.to_ne_bytes());

        let mut encode_buf = (&mut self.buf).limit(self.config.max_record_size);
        record.encode(&mut encode_buf).map_err(Error::Encode)?;

        let checksum = crc32fast::hash(&self.buf[8..]);

        let serialized_len = self.buf.len();

        // fill length delimiter and crc32
        let dst = self.buf.as_mut_slice();
        dst[..4].copy_from_slice(((serialized_len - 4) as u32).to_be_bytes().as_ref());
        dst[4..8].copy_from_slice(checksum.to_ne_bytes().as_ref());

        Ok(serialized_len)
    }

    async fn roll_to_next_writer(&mut self) -> Result<(), Error<T>> {
        let mut next_path;

        let file = loop {
            let id = self.ledger.get_next_writer_file_id();
            next_path = self.config.chunk_path(id);

            match OpenOptions::new()
                .create_new(true)
                .append(true)
                .open(&next_path)
            {
                Ok(file) => break file,
                Err(err) => {
                    match err.kind() {
                        ErrorKind::AlreadyExists => {
                            // We open the file again, without the atomic "create new" behavior.
                            // If we can do that successfully, we check its length. There's three
                            // main situations we encounter:
                            //
                            // - the reader may have deleted the data file between the atomic
                            //   create open and this one, and so we would expect the file length
                            //   to be zero
                            // - the file still exists, and it's full: the reader may still be
                            //   reading it, or waiting for acknowledgements to be able to delete
                            //   it.
                            // - it may not be full, which could be because it's the chunk file
                            //   the writer left off on last time
                            let file = OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&next_path)?;
                            let size = file.metadata()?.len();
                            if size == 0 {
                                // The file is either empty, which means we created it and "own"
                                // it now, or it's not empty but we're not skipping to the next
                                // file, which can only mean that we're still initializing, and
                                // so this would be the chunk file we left off writing to.
                                break file;
                            }

                            // The file isn't empty, and we're not in initialization anymore,
                            // which means this chunk file is one that the reader still hasn't
                            // finished reading through yet, and so we must wait for the reader
                            // to delete it before we can proceed
                            debug!(
                                message = "target chunk file is still present and not yet processed, waiting for reader",
                                path = ?next_path,
                                size
                            );

                            self.ledger.wait_for_write().await;
                        }
                        // Legitimate I/O error with the operation, bubble this up
                        _ => return Err(err.into()),
                    }
                }
            }
        };

        debug!(
            message = "current chunk file reached maximum size, roll to next chunk file",
            current_size = self.write_size,
            max_chunk_size = self.config.max_chunk_size,
            next_chunk = ?next_path,
        );

        self.flush()?;
        self.inner = file;
        self.write_size = 0;

        self.ledger.increase_writer_file_id();
        self.ledger.flush()?;

        Ok(())
    }

    /// Flushes the writer
    #[cfg_attr(test, tracing::instrument(skip(self)))]
    pub fn flush(&mut self) -> std::io::Result<()> {
        trace!(
            message = "flushing chunk file",
            chunk = self.ledger.get_current_writer_file_id(),
            records = self.unflushed_records,
            bytes = self.unflushed_bytes,
        );

        self.unflushed_bytes = 0;
        self.unflushed_records = 0;

        self.inner.sync_data()
    }
}

#[cfg(test)]
impl<T> Writer<T> {
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn ledger(&self) -> Arc<Ledger> {
        Arc::clone(&self.ledger)
    }
}
