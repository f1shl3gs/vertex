use crate::request::virNetMessageError;
use std::io;

#[derive(Debug)]
pub enum Error {
    /// IO error
    IO(std::io::Error),
    /// Error during serialization / deserialization
    Xdr(xdr_codec::Error),
    /// Libvirt returned error
    Libvirt(virNetMessageError),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::IO(err)
    }
}

impl From<xdr_codec::Error> for Error {
    fn from(err: xdr_codec::Error) -> Self {
        Self::Xdr(err)
    }
}

impl From<virNetMessageError> for Error {
    fn from(err: virNetMessageError) -> Self {
        Self::Libvirt(err)
    }
}
