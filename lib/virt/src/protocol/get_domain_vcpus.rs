use std::io::{Read, Write};

use super::{
    impl_procedure, unpack_flex, unpack_opaque_flex, Domain, Pack, ReadExt, Result, Unpack,
};

pub const REMOTE_VCPUINFO_MAX: usize = 16384;
// pub const REMOTE_CPUMAPS_MAX: usize = 8388608;

pub struct GetDomainVcpusRequest<'a> {
    pub domain: &'a Domain,
    pub max_info: i32,
    pub map_len: i32,
}

impl_procedure!(GetDomainVcpusRequest<'_>, REMOTE_PROC_DOMAIN_GET_VCPUS);

impl<W: Write> Pack<W> for GetDomainVcpusRequest<'_> {
    fn pack(&self, w: &mut W) -> Result<usize> {
        Ok(self.domain.pack(w)? + self.max_info.pack(w)? + self.map_len.pack(w)?)
    }
}

pub struct VcpuInfo {
    pub number: u32,
    pub state: i32,
    pub cpu_time: u64,
    pub cpu: i32,
}

impl<R: Read> Unpack<R> for VcpuInfo {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let number = r.read_u32()?;
        let state = r.read_i32()?;
        let cpu_time = r.read_u64()?;
        let cpu = r.read_i32()?;

        Ok((
            Self {
                number,
                state,
                cpu_time,
                cpu,
            },
            20,
        ))
    }
}

pub struct GetDomainVcpusResponse {
    pub infos: Vec<VcpuInfo>,
    pub maps: Vec<u8>,
}

impl<R: Read> Unpack<R> for GetDomainVcpusResponse {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let (infos, s1) = unpack_flex(r, REMOTE_VCPUINFO_MAX)?;
        let (maps, s2) = unpack_opaque_flex(r, REMOTE_VCPUINFO_MAX)?;

        Ok((Self { infos, maps }, s1 + s2))
    }
}
