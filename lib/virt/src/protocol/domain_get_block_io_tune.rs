use crate::protocol::Params;
use std::io::{Read, Write};

use super::constants::REMOTE_PROC_DOMAIN_GET_BLOCK_IO_TUNE;
use super::{impl_procedure, unpack_flex, Domain, Pack, RemoteTypedParam, Result, Unpack};

pub const REMOTE_DOMAIN_BLOCK_IO_TUNE_PARAMETERS_MAX: usize = 32;

pub struct GetDomainBlockIoTuneRequest<'a> {
    pub domain: &'a Domain,
    pub disk: &'a str,
    pub nparams: i32,
    pub flags: u32,
}

impl_procedure!(
    GetDomainBlockIoTuneRequest<'_>,
    REMOTE_PROC_DOMAIN_GET_BLOCK_IO_TUNE
);

impl<W: Write> Pack<W> for GetDomainBlockIoTuneRequest<'_> {
    fn pack(&self, w: &mut W) -> Result<usize> {
        Ok(self.domain.pack(w)?
            + self.disk.pack(w)?
            + self.nparams.pack(w)?
            + self.flags.pack(w)?)
    }
}

pub struct BlockIoTuneParameters {
    pub total_bytes_sec: u64,
    pub read_bytes_sec: u64,
    pub write_bytes_sec: u64,
    pub total_iops_sec: u64,
    pub read_iops_sec: u64,
    pub write_iops_sec: u64,
    pub total_bytes_sec_max: u64,
    pub read_bytes_sec_max: u64,
    pub write_bytes_sec_max: u64,
    pub total_iops_sec_max: u64,
    pub read_iops_sec_max: u64,
    pub write_iops_sec_max: u64,
    pub total_bytes_sec_max_length: u64,
    pub read_bytes_sec_max_length: u64,
    pub write_bytes_sec_max_length: u64,
    pub total_iops_sec_max_length: u64,
    pub read_iops_sec_max_length: u64,
    pub write_iops_sec_max_length: u64,
    pub size_iops_sec: u64,
}

pub struct GetDomainBlockIoTuneResponse {
    params: Vec<RemoteTypedParam>,
    pub nparams: i32,
}

impl GetDomainBlockIoTuneResponse {
    pub fn block_io_tune_parameters(self) -> BlockIoTuneParameters {
        let params = self.params;

        BlockIoTuneParameters {
            total_bytes_sec: params.get_u64("total_bytes_sec").unwrap_or_default(),
            read_bytes_sec: params.get_u64("read_bytes_sec").unwrap_or_default(),
            write_bytes_sec: params.get_u64("write_bytes_sec").unwrap_or_default(),
            total_iops_sec: params.get_u64("total_iops_sec").unwrap_or_default(),
            read_iops_sec: params.get_u64("read_iops_sec").unwrap_or_default(),
            write_iops_sec: params.get_u64("write_iops_sec").unwrap_or_default(),
            total_bytes_sec_max: params.get_u64("total_bytes_sec_max").unwrap_or_default(),
            read_bytes_sec_max: params.get_u64("read_bytes_sec_max").unwrap_or_default(),
            write_bytes_sec_max: params.get_u64("write_bytes_sec_max").unwrap_or_default(),
            total_iops_sec_max: params.get_u64("total_iops_sec_max").unwrap_or_default(),
            read_iops_sec_max: params.get_u64("read_iops_sec_max").unwrap_or_default(),
            write_iops_sec_max: params.get_u64("write_iops_sec_max").unwrap_or_default(),
            total_bytes_sec_max_length: params
                .get_u64("total_bytes_sec_max_length")
                .unwrap_or_default(),
            read_bytes_sec_max_length: params
                .get_u64("read_bytes_sec_max_length")
                .unwrap_or_default(),
            write_bytes_sec_max_length: params
                .get_u64("write_bytes_sec_max_length")
                .unwrap_or_default(),
            total_iops_sec_max_length: params
                .get_u64("total_iops_sec_max_length")
                .unwrap_or_default(),
            read_iops_sec_max_length: params
                .get_u64("read_iops_sec_max_length")
                .unwrap_or_default(),
            write_iops_sec_max_length: params
                .get_u64("write_iops_sec_max_length")
                .unwrap_or_default(),
            size_iops_sec: params.get_u64("size_iops_sec").unwrap_or_default(),
        }
    }
}

impl<R: Read> Unpack<R> for GetDomainBlockIoTuneResponse {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let (params, s1) = unpack_flex(r, REMOTE_DOMAIN_BLOCK_IO_TUNE_PARAMETERS_MAX)?;
        let (nparams, s2) = Unpack::unpack(r)?;

        Ok((Self { params, nparams }, s1 + s2))
    }
}
