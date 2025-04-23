mod acknowledgement;
mod ledger;
mod reader;
mod writer;

mod record;
#[cfg(test)]
pub mod tests;

pub use acknowledgement::{EligibleMarker, EligibleMarkerLength, OrderedAcknowledgements};
pub use ledger::{Error as LedgerError, Ledger};
pub use reader::{Error as ReaderError, Reader};
pub use writer::{Error as WriterError, Writer};

use std::path::PathBuf;
use std::sync::Arc;

use crate::Encodable;
use crate::config::Error;

/// Disk buffer configuration
#[derive(Clone, Debug)]
pub struct Config {
    /// Directory where this buffer will write its files
    ///
    /// Must be unique from all other buffers, whether within the same process
    /// or other vertex processes on the machine.
    pub root: PathBuf,

    /// Maximum size, in bytes, of an encoded record
    ///
    /// Any record which, when encoded and serialized, is larger than this amount
    /// will not be written to the buffer.
    pub max_record_size: usize,

    /// Maximum size, in bytes, to target for each individual chunk file.
    pub max_chunk_size: usize,

    /// Maximum size, in bytes, that the buffer can consume.
    ///
    /// The actual maximum on-disk buffer size is this amount rounded up to the next
    /// multiple of `max_chunk_size`, but internally, the next multiple of `max_chunk_size`
    /// when round this amount _down_ is what gets used as the maximum buffer size.
    ///
    /// This ensures that we
    pub max_buffer_size: usize,
}

impl Config {
    fn chunk_path(&self, id: u16) -> PathBuf {
        let name = format!("{:04x}.chunk", id);
        self.root.join(name)
    }

    pub fn build<T: Encodable>(self) -> Result<(Writer<T>, Reader<T>), Error> {
        std::fs::create_dir_all(&self.root)
            .map_err(|err| Error::CreateRootDirectory(self.root.clone(), err))?;

        let ledger = Ledger::create_or_load(self.root.clone())
            .map(Arc::new)
            .map_err(Error::Ledger)?;

        let writer = Writer::new(self.clone(), Arc::clone(&ledger)).map_err(Error::Io)?;
        let reader = Reader::new(self, ledger).map_err(Error::Io)?;

        Ok((writer, reader))
    }
}
