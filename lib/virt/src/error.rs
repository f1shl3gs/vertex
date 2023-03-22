use std::io;

use crate::protocol;

#[derive(Debug)]
pub enum Error {
    /// IO error
    IO(io::Error),

    /// Error during serialization / deserialization
    Xdr(protocol::Error),

    /// Libvirt returned error
    Libvirt(protocol::MessageError),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::IO(err)
    }
}

impl From<protocol::Error> for Error {
    fn from(err: protocol::Error) -> Self {
        Self::Xdr(err)
    }
}

impl From<protocol::MessageError> for Error {
    fn from(err: protocol::MessageError) -> Self {
        Self::Libvirt(err)
    }
}
