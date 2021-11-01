mod stat;
mod inotify;

use std::io;
use bytes::Bytes;
use crate::Position;

pub struct Watcher {
    position: Position
}

impl Watcher {
    #[inline]
    pub fn should_read(&self) -> bool {
        // TODO: implement this
        true
    }

    /// Read a single line from the underlying file
    ///
    /// This function will attempt to read a new line from its file, blocking up to some
    /// maximum but unspecified amount of time. `read_line` will open a new file handler
    /// as needed, transparently to the caller
    pub fn read_line(&mut self) -> io::Result<Option<Bytes>> {
        todo!()
    }

    pub fn file_position(&self) -> Position {
        self.position
    }
}