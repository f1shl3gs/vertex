use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, AtomicUsize, Ordering};

use memmap2::MmapMut;
use tokio::sync::Notify;
use tracing::{debug, error};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to create lock file {0:?} failed, {1}")]
    CreateLock(PathBuf, std::io::Error),

    /// The ledger is already opened by another process
    ///
    /// Advisory locking is used to prevent other vertex processes from concurrently
    /// opening the same buffer, but bear in mind that this does not prevent other
    /// processes or users from modifying the ledger file in a way that could cause
    /// undefined behavior during buffer operation.
    #[error("failed to lock ledger.lock")]
    LockAlreadyHeld(#[from] std::fs::TryLockError),

    #[error("mmap ledger state failed, {0}")]
    Mmap(std::io::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Ledger state, Stores the relevant information related to both the reader and writer.
/// Get serialized and stored on disk, and is managed via a memory-mapped file.
#[derive(Debug)]
#[repr(C)]
pub struct LedgerState {
    /// The current chunk file ID being written to.
    writer_current_chunk_file: AtomicU16,
    /// The current chunk file ID being read from.
    reader_current_chunk_file: AtomicU16,
    _padding: u32,

    /// Next record ID to use when writing a record.
    writer_next_record: AtomicU64,
    /// The last record ID read by the reader.
    reader_last_record: AtomicU64,
}

impl LedgerState {
    #[inline]
    pub fn get_next_write_record_id(&self) -> u64 {
        self.writer_next_record.load(Ordering::Acquire)
    }

    #[inline]
    pub fn get_last_read_record_id(&self) -> u64 {
        self.reader_last_record.load(Ordering::Acquire)
    }

    #[inline]
    pub fn get_current_reader_file_id(&self) -> u16 {
        if cfg!(test) {
            self.reader_current_chunk_file.load(Ordering::Acquire) % 8
        } else {
            self.reader_current_chunk_file.load(Ordering::Acquire)
        }
    }

    #[inline]
    pub fn get_current_writer_file_id(&self) -> u16 {
        if cfg!(test) {
            self.writer_current_chunk_file.load(Ordering::Acquire) % 8
        } else {
            self.writer_current_chunk_file.load(Ordering::Acquire)
        }
    }
}

#[cfg(test)]
impl LedgerState {
    #[inline]
    pub fn set_writer_next_record_id(&self, id: u64) {
        self.writer_next_record.store(id, Ordering::Release);
    }

    pub fn set_reader_last_record_id(&self, id: u64) {
        self.reader_last_record.store(id, Ordering::Release);
    }
}

/// Tracks the internal state of the `Buffer`
pub struct Ledger {
    /// Ledger state
    backing: MmapMut,
    /// Advisory lock for this buffer directory
    #[allow(dead_code)]
    lockfile: File,

    /// Notify for reader-related progress
    read_notify: Notify,
    /// Notify for write-related progress
    write_notify: Notify,
    /// Tracks when writer has fully shutdown
    write_done: AtomicBool,

    /// The total size, in bytes, of all unread records in the buffer.
    buffer_bytes: AtomicUsize,
    /// The total records of all unread records in the buffer
    buffer_records: AtomicUsize,
}

impl Debug for Ledger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ledger")
            .field("state", self.state())
            .field("done", &self.write_done)
            .field("buffer_bytes", &self.buffer_bytes)
            .field("buffer_records", &self.buffer_records)
            .finish_non_exhaustive()
    }
}

impl Ledger {
    /// Create or load a ledger for the given path
    pub fn create_or_load(root: PathBuf) -> Result<Self, Error> {
        let path = root.join("ledger.lock");
        let lockfile = File::create(&path).map_err(|err| Error::CreateLock(path, err))?;

        lockfile.try_lock()?;

        let path = root.join("ledger.state");
        let mut state_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        let backing = unsafe { MmapMut::map_mut(&state_file) }.map_err(Error::Mmap)?;

        // If we just create the ledger file, then we need to create the default ledger
        // state, and then serialize and write to the file, before trying to load it as
        // a memory-mapped file.
        let state_size = state_file.metadata()?.len();
        if state_size == 0 {
            debug!(
                message = "ledger.state is empty, initializing with default ledger state",
                ?path
            );

            state_file.write_all(&[
                0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
                0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            ])?;
            state_file.sync_all()?;
        }

        let state: &LedgerState = unsafe {
            let data = backing.as_ref();
            &*data.as_ptr().cast()
        };

        let buffered_records =
            state.get_next_write_record_id() - state.get_last_read_record_id() - 1;

        // Under normal operation, the reader and writer maintain a consistent state within
        // the ledger. However, due to the nature of how we update the ledger, process crashes
        // could lead to missed updates as we execute reads and writes as non-atomic units of
        // execution: update a field, do the read/write, update some more fields depending on
        // success or failure, etc.
        //
        // This is an issue because we depend on knowing the total buffer size (the total size
        // of unread records, specifically) so that we can correctly limit writes when we've
        // reached the configured maximum buffer size.
        //
        // While it's not terribly efficient, and I'd like to eventually formulate a better
        // design, this approach is absolutely correct: get the file size of every chunk file
        // on disk, and set the "total buffer size" to the sum of all of those file sizes.
        //
        // When the reader does any necessary seeking to get to the record it left off on,
        // it will adjust the "total buffer size" downwards for each record it runs through,
        // leaving "total buffer size" at the correct value.
        let mut total_buffer_size = 0;
        for entry in std::fs::read_dir(root)?.flatten() {
            let path = entry.path();

            if matches!(path.extension(), Some(ext) if ext == "chunk") {
                let size = path.metadata()?.len();
                total_buffer_size += size;

                debug!(
                    message = "found existing chunk file",
                    ?path,
                    size,
                    total_buffer_size,
                );
            }
        }

        Ok(Ledger {
            backing,
            lockfile,
            buffer_bytes: AtomicUsize::new(total_buffer_size as usize),
            buffer_records: AtomicUsize::new(buffered_records as usize),
            read_notify: Notify::default(),
            write_notify: Notify::default(),
            write_done: AtomicBool::default(),
        })
    }

    /// Notifies all tasks waiting on progress by the reader.
    #[cfg_attr(test, tracing::instrument(skip(self)))]
    pub fn notify_readers(&self) {
        self.read_notify.notify_one();
    }

    /// Waits for a signal from the reader that progress has been made.
    ///
    /// This will only occur when a record is read, which may allow enough space
    /// (below the maximum configured buffer size) for a write to occur, or similarly,
    /// when a chunk file is deleted.
    #[cfg_attr(test, tracing::instrument(skip(self)))]
    pub async fn wait_for_read(&self) {
        self.read_notify.notified().await;
    }

    /// Notifies all tasks waiting on progress by the writer.
    #[cfg_attr(test, tracing::instrument(skip(self)))]
    pub fn notify_writers(&self) {
        self.write_notify.notify_one();
    }

    /// Wait for a signal from the writer that progress has been made.
    ///
    /// This will occur when a record is written, or when a new chunk file is created.
    #[cfg_attr(test, tracing::instrument(skip(self)))]
    pub async fn wait_for_write(&self) {
        self.write_notify.notified().await;
    }

    /// Gets the internal ledger state.
    ///
    /// This is the information persisted to disk
    pub fn state(&self) -> &LedgerState {
        let data = self.backing.as_ref();
        unsafe { &*data.as_ptr().cast() }
    }

    #[cfg_attr(test, tracing::instrument(skip(self)))]
    #[inline]
    pub fn flush(&self) -> std::io::Result<()> {
        self.backing.flush()
    }

    /// Gets the current reader file ID
    ///
    /// This is internally adjusted to compensate for the fact that the reader can read
    /// far past the latest acknowledge record/data file, and so is not representative
    /// of where the reader would start reading from if the process crashed or was abruptly
    /// stopped.
    #[inline]
    pub fn get_current_reader_file_id(&self) -> u16 {
        self.state().get_current_reader_file_id()
    }

    #[inline]
    pub fn increment_reader_file_id(&self) -> u16 {
        self.state()
            .reader_current_chunk_file
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1)
    }

    #[inline]
    pub fn get_current_writer_file_id(&self) -> u16 {
        self.state().get_current_writer_file_id()
    }

    #[inline]
    pub fn get_next_writer_file_id(&self) -> u16 {
        if cfg!(test) {
            (self.state().get_current_writer_file_id() + 1) % 8
        } else {
            self.state().get_current_writer_file_id() + 1
        }
    }

    pub fn get_next_write_record_id(&self) -> u64 {
        self.state().writer_next_record.load(Ordering::Acquire)
    }

    #[inline]
    pub fn get_last_read_record_id(&self) -> u64 {
        self.state().get_last_read_record_id()
    }

    #[inline]
    pub fn increase_last_reader_record_id(&self, amount: u64) {
        self.state()
            .reader_last_record
            .fetch_add(amount, Ordering::AcqRel);
    }

    #[inline]
    pub fn increase_writer_file_id(&self) {
        self.state()
            .writer_current_chunk_file
            .fetch_add(1, Ordering::Release);
    }

    /// Gets the total number of bytes for all unread records in the buffer.
    ///
    /// This number will often disagree
    pub fn get_buffer_bytes(&self) -> usize {
        self.buffer_bytes.load(Ordering::Acquire)
    }

    pub fn increase_buffer_bytes(&self, amount: usize) {
        self.buffer_bytes.fetch_add(amount, Ordering::AcqRel);
    }

    pub fn decrease_buffer_bytes(&self, amount: usize) {
        self.buffer_bytes.fetch_sub(amount, Ordering::AcqRel);
    }

    pub fn get_buffer_records(&self) -> usize {
        self.buffer_records.load(Ordering::Acquire)
    }

    pub fn increase_buffer_records(&self, amount: usize) {
        self.buffer_records.fetch_add(amount, Ordering::AcqRel);
    }

    pub fn decrease_buffer_records(&self, amount: usize) {
        self.buffer_records.fetch_sub(amount, Ordering::AcqRel);
    }

    /// Marks the writer as finished
    ///
    /// If the writer was not yet marked done, `false` is returned. Otherwise, `true` is
    /// returned and the caller should handle any necessary logic for closing the writer.
    pub fn mark_done(&self) -> bool {
        self.write_done
            .compare_exchange_weak(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Return `true` if the writer was marked as done
    pub fn done(&self) -> bool {
        self.write_done.load(Ordering::Acquire)
    }

    pub fn increase_next_write_record_id(&self, amount: u64) -> u64 {
        let prev = self
            .state()
            .writer_next_record
            .fetch_add(amount, Ordering::AcqRel);

        prev.wrapping_add(amount)
    }

    #[inline]
    pub fn last_reader_record_id(&self) -> u64 {
        self.state().reader_last_record.load(Ordering::Acquire)
    }

    pub fn track_write(&self, amount: usize) {
        self.state()
            .writer_next_record
            .fetch_add(1, Ordering::AcqRel);

        self.buffer_records.fetch_add(1, Ordering::AcqRel);
        self.buffer_bytes.fetch_add(amount, Ordering::AcqRel);
    }

    pub fn track_acknowledgement(&self, amount: usize) {
        self.state()
            .reader_last_record
            .fetch_add(1, Ordering::AcqRel);

        self.buffer_records.fetch_sub(1, Ordering::AcqRel);
        self.buffer_bytes.fetch_sub(amount, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use rand::distr::Alphanumeric;

    #[test]
    fn size() {
        assert_eq!(size_of::<LedgerState>(), 24);
    }

    #[tokio::test]
    async fn crud() {
        let mut rng = rand::rng();
        let dir = (&mut rng)
            .sample_iter(Alphanumeric)
            .take(8)
            .map(char::from)
            .collect::<String>();

        let path = std::env::temp_dir().join(dir);
        std::fs::create_dir_all(&path).unwrap();

        {
            let ledger = Ledger::create_or_load(path.clone()).unwrap();

            let state = ledger.state();
            assert_eq!(state.reader_current_chunk_file.load(Ordering::SeqCst), 0);
            assert_eq!(state.writer_current_chunk_file.load(Ordering::SeqCst), 0);
            assert_eq!(state.reader_last_record.load(Ordering::SeqCst), 0);
            assert_eq!(state.writer_next_record.load(Ordering::SeqCst), 1);

            ledger
                .state()
                .reader_current_chunk_file
                .store(1, Ordering::Release);
            assert_eq!(
                ledger
                    .state()
                    .reader_current_chunk_file
                    .load(Ordering::SeqCst),
                1
            );
            assert_eq!(
                ledger
                    .state()
                    .writer_current_chunk_file
                    .load(Ordering::SeqCst),
                0
            );
            assert_eq!(ledger.state().reader_last_record.load(Ordering::SeqCst), 0);
            assert_eq!(ledger.state().writer_next_record.load(Ordering::SeqCst), 1);

            ledger
                .state()
                .writer_current_chunk_file
                .store(2, Ordering::Release);
            assert_eq!(state.reader_current_chunk_file.load(Ordering::SeqCst), 1);
            assert_eq!(state.writer_current_chunk_file.load(Ordering::SeqCst), 2);
            assert_eq!(state.reader_last_record.load(Ordering::SeqCst), 0);
            assert_eq!(state.writer_next_record.load(Ordering::SeqCst), 1);

            ledger
                .state()
                .reader_last_record
                .store(3, Ordering::Release);
            assert_eq!(state.reader_current_chunk_file.load(Ordering::SeqCst), 1);
            assert_eq!(state.writer_current_chunk_file.load(Ordering::SeqCst), 2);
            assert_eq!(state.reader_last_record.load(Ordering::SeqCst), 3);
            assert_eq!(state.writer_next_record.load(Ordering::SeqCst), 1);

            ledger
                .state()
                .writer_next_record
                .store(4, Ordering::Release);
            assert_eq!(state.reader_current_chunk_file.load(Ordering::SeqCst), 1);
            assert_eq!(state.writer_current_chunk_file.load(Ordering::SeqCst), 2);
            assert_eq!(state.reader_last_record.load(Ordering::SeqCst), 3);
            assert_eq!(state.writer_next_record.load(Ordering::SeqCst), 4);
        }

        let ledger = Ledger::create_or_load(path).unwrap();
        let state = ledger.state();
        assert_eq!(state.reader_current_chunk_file.load(Ordering::SeqCst), 1);
        assert_eq!(state.writer_current_chunk_file.load(Ordering::SeqCst), 2);
        assert_eq!(state.reader_last_record.load(Ordering::SeqCst), 3);
        assert_eq!(state.writer_next_record.load(Ordering::SeqCst), 4);
    }
}
