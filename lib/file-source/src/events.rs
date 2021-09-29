use std::{
    path::Path,
    io::Error,
    time::Duration
};

/// Every internal event in this crate has a corresponding
/// method in this trait which should emit the event.
pub trait InternalEvents: Send + Sync + Clone + 'static {
    fn emit_file_added(&self, path: &Path);

    fn emit_file_resumed(&self, path: &Path, pos: u64);

    fn emit_file_watch_failed(&self, path: &Path, error: Error);

    fn emit_file_unwatched(&self, path: &Path);

    fn emit_file_deleted(&self, path: &Path);

    fn emit_file_delete_failed(&self, path: &Path, error: Error);

    fn emit_file_fingerprint_read_failed(&self, path: &Path, error: Error);

    fn emit_file_checkpointed(&self, count: usize, duration: Duration);

    fn emit_file_checksum_failed(&self, path: &Path);

    fn emit_file_checkpoint_write_failed(&self, error: Error);

    fn emit_files_open(&self, count: usize);

    fn emit_path_globbing_failed(&self, path: &Path, error: &Error);
}
