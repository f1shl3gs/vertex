use std::io::{Read, Write};

use super::{impl_procedure, Pack, Result, Unpack};

pub struct Pool {
    pub name: String,
    pub uuid: [u8; 16],
}

impl<W: Write> Pack<W> for Pool {
    fn pack(&self, w: &mut W) -> crate::protocol::Result<usize> {
        Ok(self.name.pack(w)? + self.uuid.pack(w)?)
    }
}

impl<R: Read> Unpack<R> for Pool {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let mut sz = 0;

        Ok((
            Self {
                name: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                uuid: {
                    let mut buf = [0u8; 16];
                    r.read_exact(&mut buf)?;
                    sz += 16;
                    buf
                },
            },
            sz,
        ))
    }
}

pub struct GetStoragePoolInfoRequest<'a> {
    pub pool: &'a Pool,
}

impl_procedure!(
    GetStoragePoolInfoRequest<'_>,
    REMOTE_PROC_STORAGE_POOL_GET_INFO
);

impl<W: Write> Pack<W> for GetStoragePoolInfoRequest<'_> {
    fn pack(&self, w: &mut W) -> Result<usize> {
        self.pool.pack(w)
    }
}

pub struct GetStoragePoolInfoResponse {
    pub state: u8,
    pub capacity: u64,
    pub allocation: u64,
    pub available: u64,
}

impl<R: Read> Unpack<R> for GetStoragePoolInfoResponse {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let mut sz = 0;

        Ok((
            Self {
                state: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                capacity: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                allocation: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                available: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}
