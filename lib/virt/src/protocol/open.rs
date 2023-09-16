use std::io::{Read, Write};

use crate::protocol::{
    impl_procedure, pack_string, Pack, Result, Unpack, WriteExt, VIR_NET_MESSAGE_STRING_MAX,
};

pub struct OpenRequest {
    pub name: String,
    pub flags: u32,
}

impl Default for OpenRequest {
    fn default() -> Self {
        Self {
            name: "qemu:///system".to_string(),
            flags: 0,
        }
    }
}

impl_procedure!(OpenRequest, REMOTE_PROC_CONNECT_OPEN);

impl<W: Write> Pack<W> for OpenRequest {
    fn pack(&self, w: &mut W) -> Result<usize> {
        let sz = pack_string(&self.name, Some(VIR_NET_MESSAGE_STRING_MAX), w)?;
        w.write_u32(self.flags)?;
        Ok(sz + 4)
    }
}

pub struct OpenResponse {}

impl<R: Read> Unpack<R> for OpenResponse {
    fn unpack(_r: &mut R) -> Result<(Self, usize)> {
        Ok((OpenResponse {}, 0))
    }
}
