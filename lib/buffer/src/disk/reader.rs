use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::future::Pending;
use std::io::Read;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use finalize::{BatchNotifier, BatchStatus, OrderedFinalizer};
use futures::stream::BoxStream;
use futures::{Stream, StreamExt};
use tracing::{debug, error, trace, warn};

use super::record::{Error as RecordError, RecordStatus, seek_to_next_record_id, validate_record};
use super::{Config, EligibleMarker, EligibleMarkerLength, Ledger, OrderedAcknowledgements};
use crate::Encodable;

/// Error that occurred during calls to [`Reader`]
#[derive(Debug, thiserror::Error)]
pub enum Error<T: Encodable> {
    /// A general I/O error occurred.
    ///
    /// Different methods will capture specific I/O errors depending on the situation,
    /// as some errors may be expected and considered normal by design. For all I/O
    /// errors that considered atypical, they will be returned as this variant.
    #[error("read I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The record's checksum did not match
    ///
    /// In most cases, this indicates that the chunk file being read was corrupted or
    /// truncated in some fashion. Callers of [`Reader::next`] will not actually receive
    /// this error, as it is handled internally by moving to the next chunk file, as
    /// corruption may have affected other records in a way that is not easily detectable
    /// and could lead to records which decode but contain invalid data.
    #[error("calculated checksum did not match the actual checksum, {calculated} != {actual}")]
    Checksum { calculated: u32, actual: u32 },

    /// The decoder encountered an issue during decoding.
    #[error("failed to decoded record from buffer: {0}")]
    Decode(<T as Encodable>::Error),

    /// The reader detected that a chunk file contains a partially-written record.
    ///
    /// Records should never be partially written to a chunk file ( we don't split records
    /// across chunk files) so this would be indicative of a write that was never properly
    /// written/flushed, or some issue with the write where it was acknowledged but the
    /// chunk/file was corrupted in some way.
    ///
    /// This is effectively the same class of error as an invalid checksum/failed deserialization.
    #[error("partial write detected")]
    PartialWrite,
}

struct BufReader {
    file: File,
    buf: Vec<u8>,
    start: usize,
    end: usize,
}

impl Debug for BufReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufReader")
            .field("capacity", &self.capacity())
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}

impl BufReader {
    const INITIAL_SIZE: usize = 16 * 1024;

    fn new(file: File) -> Self {
        Self {
            file,
            buf: vec![0u8; Self::INITIAL_SIZE],
            start: 0,
            end: 0,
        }
    }

    #[inline]
    fn get_mut(&mut self) -> &mut File {
        &mut self.file
    }

    #[inline]
    fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// discard drops the unused buffer, and return the actual amount of discard bytes
    #[allow(dead_code)]
    fn discard(&mut self, amount: usize) -> usize {
        let len = self.len();

        if amount < len {
            self.start += amount;
            amount
        } else {
            self.start = 0;
            self.end = 0;

            if amount > len { amount - len } else { amount }
        }
    }

    fn grow(&mut self) {
        if self.len() == self.buf.len() {
            // this buffer is already full, double its size
            self.buf.reserve(self.buf.len());

            // It's totally fine
            #[allow(clippy::uninit_vec)]
            unsafe {
                self.buf.set_len(self.buf.len() * 2)
            };
        } else if self.end == self.buf.len() {
            // there is still some room, filling existing buffer
            if self.len() != 0 && self.start != 0 {
                self.buf.copy_within(self.start..self.end, 0);

                self.end -= self.start;
                self.start = 0;
            }
        }

        // there's still some room in `unfilled()`, nothing to do
    }

    #[inline]
    fn len(&self) -> usize {
        self.end - self.start
    }

    fn fill_buf(&mut self) -> std::io::Result<usize> {
        let unfilled = self.buf.capacity() - self.end;
        if unfilled == 0 {
            self.grow();
        }

        let uf = &mut self.buf[self.end..];
        let amount = self.file.read(uf)?;
        self.end += amount;

        Ok(amount)
    }

    fn consume(&mut self, amount: usize) -> &[u8] {
        assert!(self.start + amount <= self.end);

        let amount = std::cmp::min(amount, self.len());
        let start = self.start;
        self.start += amount;
        if self.start == self.end {
            self.start = 0;
            self.end = 0;
        }

        &self.buf[start..(start + amount)]
    }
}

type Acknowledgements = Arc<OrderedAcknowledgements<(usize, Option<PathBuf>)>>;

/// Reads records from the chunk file
///
/// Reader not just provide a mechanism to read record form chunk files, it also tracks the
/// acknowledgments of records and chunk files.
///
/// - If the chunk file cannot provide any record and the writer_file_id != reader_file_id,
///   the reader will roll to next chunk file
/// - Once the chunk file's all records are acknowledgment the chunk file would be deleted.
///
/// ```text
///       last acknowledgement      Writer
///              |                    |
///              V                    V
///   ------------------------------------
///                      ^
///                      |
///                    Reader
/// ```
pub struct Reader<T> {
    config: Config,
    ledger: Arc<Ledger>,

    inner: BufReader,
    finalizer: OrderedFinalizer<u64>,

    // TODO: drop mutex with AtomicU64 and Queue
    records_acknowledgements: Acknowledgements,

    _pd: PhantomData<T>,
}

impl<T: Encodable> Reader<T> {
    /// Creates a new [`Reader`] attached to the given [`Ledger`]
    pub fn new(config: Config, ledger: Arc<Ledger>) -> std::io::Result<Self> {
        let last_reader_record_id = ledger.last_reader_record_id();
        let next_expected_record_id = last_reader_record_id.wrapping_add(1);

        let file = loop {
            let id = ledger.get_current_reader_file_id();
            let path = config.chunk_path(id);

            let mut file = OpenOptions::new().read(true).open(&path)?;
            // seek to the right place, so we can read the right record
            match seek_to_next_record_id(&mut file, next_expected_record_id) {
                Ok(position) => {
                    debug!(
                        message = "seek reader to right offset",
                        ?path,
                        expect = next_expected_record_id,
                        position,
                    );

                    break file;
                }
                Err(err) => match err {
                    RecordError::Io(err) => return Err(err),
                    RecordError::PartialWrite => {
                        warn!(
                            message = "partial write detected by reader, roll to next chunk",
                            ?path,
                        );

                        ledger.increment_reader_file_id();
                        ledger.decrease_buffer_bytes(file.metadata()?.len() as usize);
                    }
                    RecordError::Corrupted { .. } => unreachable!(),
                },
            }
        };

        let records_acknowledgements = Arc::new(
            OrderedAcknowledgements::<(usize, Option<PathBuf>)>::from_acked(
                next_expected_record_id,
            ),
        );

        let (finalizer, stream) = OrderedFinalizer::new::<Pending<()>>(None);
        tokio::spawn(run_finalizer(
            Arc::clone(&ledger),
            Arc::clone(&records_acknowledgements),
            stream,
        ));

        Ok(Self {
            ledger,
            config,

            inner: BufReader::new(file),
            finalizer,
            records_acknowledgements,

            _pd: PhantomData,
        })
    }

    /// Reads a record
    ///
    /// If the writer is closed and there is no more data in the buffer, `None` is returned.
    /// Otherwise, reads the next record or waits until the next record is available.
    ///
    /// # Errors
    ///
    /// If an error occurred while reading a record, an error variant will be returned describing
    /// the error.
    pub async fn read(&mut self) -> Result<Option<T>, Error<T>> {
        let mut maybe_chunk = None;

        let length = loop {
            // If the writer has marked themselves as done, and the buffer has been emptied, then
            // we're done and can return.
            if self.ledger.done() {
                let buffered = self.ledger.get_buffer_records();
                if buffered == 0 {
                    return Ok(None);
                }
            }

            let available = self.inner.len();
            if available >= 4 {
                let buf = self.inner.consume(4);
                let length =
                    u32::from_be_bytes(buf.try_into().expect("the slice is the length of a u32"))
                        as usize;

                break length;
            }

            let filled = self.inner.fill_buf()?;
            if filled == 0 {
                let reader_file_id = self.ledger.get_current_reader_file_id();
                let writer_file_id = self.ledger.get_current_writer_file_id();
                let finalized = reader_file_id != writer_file_id;

                if finalized {
                    debug!(
                        message = "reached the end of current chunk file",
                        reader_file_id, writer_file_id,
                    );

                    self.roll_to_next_chunk_file().await?;
                    maybe_chunk = Some(self.config.chunk_path(reader_file_id));
                } else {
                    self.ledger.wait_for_read().await;
                }

                continue;
            }
        };

        // the reader doesn't care about the record size, only if the size is
        // a decent value

        // ensure the record is filled to the buffer
        loop {
            let available = self.inner.len();
            if available >= length {
                break;
            }

            let filled = self.inner.fill_buf()?;
            if filled == 0 {
                // reach the file end
                let reader_file_id = self.ledger.get_current_reader_file_id();
                let writer_file_id = self.ledger.get_current_writer_file_id();

                if reader_file_id != writer_file_id {
                    // if we needed more data, but there was none available, and we're finalized,
                    // we've got ourselves a partial write situation
                    return Err(Error::PartialWrite);
                }
            }
        }

        let buf = self.inner.consume(length);
        match validate_record(buf) {
            RecordStatus::Valid { id } => {
                let mut record = T::decode(&buf[4 + 8..]).map_err(Error::Decode)?;

                if let Err(_err) = self.records_acknowledgements.add_marker(
                    id,
                    Some(1),
                    Some((4 + length, maybe_chunk)),
                ) {
                    panic!("record ID monotonicity violation detected; this is a serious bug");
                }

                let (batch, receiver) = BatchNotifier::new_with_receiver();
                record.add_batch_notifier(batch);
                self.finalizer.add(1, receiver);

                trace!(
                    message = "read record",
                    id,
                    size = length + 4,
                    chunk = self.ledger.get_current_reader_file_id()
                );

                // NB: ledger don't store reader id, but the acknowledgement id,
                // so notify here is not necessary.

                Ok(Some(record))
            }
            RecordStatus::Corrupted { calculated, actual } => {
                Err(Error::Checksum { calculated, actual })
            }
        }
    }

    #[cfg_attr(test, tracing::instrument(skip(self)))]
    async fn roll_to_next_chunk_file(&mut self) -> std::io::Result<()> {
        // Add a marker for this chunk file so we know when it can be safely deleted. We also
        // need to track the necessary data to do our buffer accounting when it's eligible
        // for deletion.
        //
        // In the rare case where the very first read in a new chunk file is corrupted/invalid
        // and we roll to the next chunk file, we simply use the last reader record ID we have,
        // which yields a marker with a length of 0

        // let current_record_id = self.ledger.get_last_read_record_id();
        // let current = self.ledger.get_current_reader_file_id();

        let next = self.ledger.increment_reader_file_id();

        let file = OpenOptions::new()
            .read(true)
            .open(self.config.chunk_path(next))?;

        *self.inner.get_mut() = file;

        debug!(message = "roll to next chunk file", chunk = next);

        Ok(())
    }
}

pin_project_lite::pin_project! {
    /// ReadyFold is used to accumulate amount of the inner stream
    #[must_use = "streams do nothing unless polled"]
    struct ReadyFold<S> {
        #[pin]
        stream: S,
    }
}

impl<S: Stream<Item = (BatchStatus, u64)>> Stream for ReadyFold<S> {
    type Item = u64;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        let mut amount = 0;
        loop {
            match this.stream.as_mut().poll_next(cx) {
                Poll::Pending => {
                    if amount == 0 {
                        return Poll::Pending;
                    }

                    return Poll::Ready(Some(amount));
                }
                Poll::Ready(None) => {
                    if amount == 0 {
                        return Poll::Ready(None);
                    }

                    return Poll::Ready(Some(amount));
                }
                Poll::Ready(Some((_status, batch))) => {
                    amount += batch;
                }
            }
        }
    }
}

async fn run_finalizer(
    ledger: Arc<Ledger>,
    acknowledgements: Acknowledgements,
    stream: BoxStream<'_, (BatchStatus, u64)>,
) {
    let mut stream = ReadyFold { stream };

    while let Some(amount) = stream.next().await {
        acknowledgements.add_acknowledgements(amount);

        let mut ack_records = 0;
        let mut ack_bytes = 0;
        while let Some(EligibleMarker { len, data, .. }) =
            acknowledgements.get_next_eligible_marker()
        {
            if let Some((bytes, maybe_chunk)) = data {
                ack_bytes += bytes;

                if let Some(path) = maybe_chunk {
                    match std::fs::remove_file(&path) {
                        Ok(_) => {
                            debug!(message = "deleting completed chunk file successful", ?path,);
                        }
                        Err(err) => {
                            error!(
                                message = "deleting completed chunk file failed",
                                ?path,
                                ?err
                            )
                        }
                    }
                }
            }

            match len {
                EligibleMarkerLength::Known(amount) => {
                    ack_records += amount;
                }
                EligibleMarkerLength::Assumed(_) => {}
            }
        }

        ledger.increase_last_reader_record_id(ack_records);
        ledger.decrease_buffer_bytes(ack_bytes);
        ledger.decrease_buffer_records(ack_records as usize);

        ledger.notify_writers();
    }
}
