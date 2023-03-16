use std::io::Write;

use super::impl_procedure;
use super::{Pack, ReadExt, Result, Unpack, REMOTE_PROC_CONNECT_GET_VERSION};

pub struct GetLibVersionRequest {}

impl_procedure!(GetLibVersionRequest, REMOTE_PROC_CONNECT_GET_VERSION);

impl<W: Write> Pack<W> for GetLibVersionRequest {
    fn pack(&self, _w: &mut W) -> Result<usize> {
        Ok(0)
    }
}

pub struct GetLibVersionResponse {
    version: u64,
}

impl GetLibVersionResponse {
    pub fn version(&self) -> String {
        version_num_to_string(self.version)
    }
}

#[inline]
pub fn version_num_to_string(v: u64) -> String {
    format!(
        "{}.{}.{}",
        v / 1000 / 1000 % 1000,
        v / 1000 % 1000,
        v % 1000
    )
}

impl<R: ReadExt> Unpack<R> for GetLibVersionResponse {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let version = r.read_u64()?;

        Ok((Self { version }, 8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::assert_pack;

    #[test]
    fn pack() {
        let req = GetLibVersionRequest {};

        assert_pack(req, &[])
    }
}
