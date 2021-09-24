mod buffer;
mod checkpointer;
mod server;
mod fingerprinter;
mod events;

pub type FilePosition = u64;

pub enum ReadFrom {
    Beginning,
    End,
    Checkpoint(FilePosition),
}

impl Default for ReadFrom {
    fn default() -> Self {
        ReadFrom::Beginning
    }
}