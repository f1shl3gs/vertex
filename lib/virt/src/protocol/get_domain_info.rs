use std::io::{Read, Write};

use super::{impl_procedure, Domain, Pack, ReadExt, Result, Unpack};

pub struct GetDomainInfoRequest<'a> {
    pub domain: &'a Domain,
}

impl_procedure!(GetDomainInfoRequest<'_>, REMOTE_PROC_DOMAIN_GET_INFO);

impl<W: Write> Pack<W> for GetDomainInfoRequest<'_> {
    fn pack(&self, w: &mut W) -> Result<usize> {
        self.domain.pack(w)
    }
}

pub type DomainState = i32;

pub struct DomainInfo {
    /// The running state, one of virDomainState.
    pub state: DomainState,
    /// The maximum memory in KBytes allowed.
    pub max_mem: u64,
    /// The memory in KBytes used by the domain.
    pub memory: u64,
    /// The number of virtual CPUs for the domain.
    pub nr_virt_cpu: u32,
    /// The CPU time used in nanoseconds.
    pub cpu_time: u64,
}

pub struct GetDomainInfoResponse {
    pub info: DomainInfo,
}

impl<R: Read> Unpack<R> for GetDomainInfoResponse {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let state = r.read_i32()? as DomainState;
        let max_mem = r.read_u64()?;
        let memory = r.read_u64()?;
        let nr_virt_cpu = r.read_u32()?;
        let cpu_time = r.read_u64()?;

        Ok((
            Self {
                info: DomainInfo {
                    state,
                    max_mem,
                    memory,
                    nr_virt_cpu,
                    cpu_time,
                },
            },
            32,
        ))
    }
}
