use std::io::{Read, Write};

use super::constants::REMOTE_PROC_DOMAIN_MEMORY_STATS;
use super::{impl_procedure, unpack_flex, Domain, Pack, ReadExt, Result, Unpack, WriteExt};

pub struct DomainMemoryStatsRequest<'a> {
    pub domain: &'a Domain,
    pub max_stats: u32,
    pub flags: u32,
}

impl_procedure!(
    DomainMemoryStatsRequest<'_>,
    REMOTE_PROC_DOMAIN_MEMORY_STATS
);

impl<W: Write> Pack<W> for DomainMemoryStatsRequest<'_> {
    fn pack(&self, w: &mut W) -> Result<usize> {
        let size = self.domain.pack(w)? + self.max_stats.pack(w)? + self.flags.pack(w)?;

        Ok(size)
    }
}

#[derive(Debug, Default)]
pub struct MemoryStats {
    pub major_fault: u64,
    pub minor_fault: u64,
    pub unused: u64,
    pub available: u64,
    pub actual_balloon: u64,
    pub rss: u64,
    pub usable: u64,
    pub disk_caches: u64,
}

struct MemoryStat {
    tag: i32,
    value: u64,
}

impl<R: Read> Unpack<R> for MemoryStat {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let tag = r.read_i32()?;
        let value = r.read_u64()?;

        Ok((Self { tag, value }, 12))
    }
}

pub struct DomainMemoryStatsResponse {
    pub stats: MemoryStats,
}

impl<R: Read> Unpack<R> for DomainMemoryStatsResponse {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        // 16 should be enough.
        let (resp, sz): (Vec<MemoryStat>, usize) = unpack_flex(r, 16)?;
        let mut stats = MemoryStats::default();

        for s in &resp {
            match s.tag {
                2 => stats.major_fault = s.value,
                3 => stats.minor_fault = s.value,
                4 => stats.unused = s.value,
                5 => stats.available = s.value,
                6 => stats.actual_balloon = s.value,
                7 => stats.rss = s.value,
                8 => stats.usable = s.value,
                10 => stats.disk_caches = s.value,
                _ => { /* do nothing */ }
            }
        }

        Ok((DomainMemoryStatsResponse { stats }, sz))
    }
}
