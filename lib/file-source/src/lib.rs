mod buffer;
mod checkpointer;
mod server;
mod fingerprinter;
mod events;

#[macro_use] extern crate scan_fmt;

pub type Position = u64;

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