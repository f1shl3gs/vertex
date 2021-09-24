use std::{
    path::Path,
    io::Error,
};

/// Every internal event in this crate has a corresponding
/// method in this trait which should emit the event.
pub trait Events: Send + Sync + Clone + 'static {
    fn emit_file_added(&self, path: &Path);

    fn emit_file_resumed(&self, path: &Path, pos: u64);

    fn emit_file_watch_failed(&self, path: &Path, error: Error);
}
