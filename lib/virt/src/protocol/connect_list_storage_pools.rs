use std::io::{Read, Write};

use super::{impl_procedure, unpack_flex, Pack, Result, Unpack};
use crate::protocol::Pool;

pub const REMOTE_STORAGE_POOL_LIST_MAX: usize = 4096;

pub struct ConnectListStoragePoolsRequest {
    pub maxnames: i32,
}

impl_procedure!(
    ConnectListStoragePoolsRequest,
    REMOTE_PROC_CONNECT_LIST_STORAGE_POOLS
);

impl<W: Write> Pack<W> for ConnectListStoragePoolsRequest {
    fn pack(&self, w: &mut W) -> Result<usize> {
        self.maxnames.pack(w)
    }
}

pub struct ConnectListStoragePoolsResponse {
    pub pools: Vec<Pool>,
    pub ret: u32,
}

impl<R: Read> Unpack<R> for ConnectListStoragePoolsResponse {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let (pools, s1) = unpack_flex(r, REMOTE_STORAGE_POOL_LIST_MAX)?;
        let (ret, s2) = Unpack::unpack(r)?;

        Ok((Self { pools, ret }, s1 + s2))
    }
}
