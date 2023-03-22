use std::io::{Read, Write};

use super::constants::REMOTE_PROC_CONNECT_GET_VERSION;
use super::{impl_procedure, version_num_to_string, Pack, ReadExt, Result, Unpack};

pub struct GetVersionRequest {}

impl_procedure!(GetVersionRequest, REMOTE_PROC_CONNECT_GET_VERSION);

impl<W: Write> Pack<W> for GetVersionRequest {
    fn pack(&self, _w: &mut W) -> Result<usize> {
        Ok(0)
    }
}

pub struct GetVersionResponse {
    version: u64,
}

impl GetVersionResponse {
    pub fn version(&self) -> String {
        version_num_to_string(self.version)
    }
}

impl<R: Read> Unpack<R> for GetVersionResponse {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let version = r.read_u64()?;

        Ok((Self { version }, 8))
    }
}
