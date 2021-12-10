mod buffer;
mod checkpointer;
mod events;
mod fingerprinter;
mod metadata_ext;
mod provider;
mod server;
mod watcher;

#[macro_use]
extern crate scan_fmt;

pub type Position = u64;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ReadFrom {
    Beginning,
    End,
    Checkpoint(Position),
}

impl Default for ReadFrom {
    fn default() -> Self {
        ReadFrom::Beginning
    }
}
