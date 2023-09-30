use std::io::{Read, Write};

use super::constants::{REMOTE_DOMAIN_LIST_MAX, VIR_NET_MESSAGE_STRING_MAX};
use super::{
    impl_procedure, pack_flex, unpack_flex, unpack_string, Domain, Error, Pack, ReadExt, Result,
    Unpack,
};

pub const REMOTE_CONNECT_GET_ALL_DOMAIN_STATS_MAX: usize = 4096;

pub enum RemoteTypedParamValue {
    Const1(i32),
    Const2(u32),
    Const3(i64),
    Const4(u64),
    Const5(f64),
    Const6(i32),
    Const7(String),
}

impl<R: Read> Unpack<R> for RemoteTypedParamValue {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        match r.read_i32()? {
            1 => {
                let v = r.read_i32()?;
                Ok((RemoteTypedParamValue::Const1(v), 4))
            }
            2 => {
                let v = r.read_u32()?;
                Ok((RemoteTypedParamValue::Const2(v), 4))
            }
            3 => {
                let v = r.read_i64()?;
                Ok((RemoteTypedParamValue::Const3(v), 8))
            }
            4 => {
                let v = r.read_u64()?;
                Ok((RemoteTypedParamValue::Const4(v), 8))
            }
            5 => {
                let v = r.read_f64()?;
                Ok((RemoteTypedParamValue::Const5(v), 8))
            }
            6 => {
                let v = r.read_i32()?;
                Ok((RemoteTypedParamValue::Const6(v), 4))
            }
            7 => {
                let (data, sz) = unpack_string(r, VIR_NET_MESSAGE_STRING_MAX)?;
                Ok((RemoteTypedParamValue::Const7(data), sz))
            }
            n => Err(Error::InvalidEnum(n)),
        }
    }
}

pub struct RemoteTypedParam {
    field: String,
    value: RemoteTypedParamValue,
}

impl RemoteTypedParam {
    pub fn as_u32(&self) -> u32 {
        match self.value {
            RemoteTypedParamValue::Const2(v) => v,
            _ => panic!(),
        }
    }

    pub fn as_u64(&self) -> u64 {
        match self.value {
            RemoteTypedParamValue::Const4(v) => v,
            _ => panic!(),
        }
    }

    pub fn as_string(&self) -> String {
        match self.value {
            RemoteTypedParamValue::Const7(ref s) => s.clone(),
            _ => panic!(),
        }
    }
}

impl<R: Read> Unpack<R> for RemoteTypedParam {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let mut size = 0;
        Ok((
            Self {
                field: {
                    let (v, fsz) = unpack_string(r, VIR_NET_MESSAGE_STRING_MAX)?;
                    size += fsz;
                    v
                },
                value: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    size += fsz;
                    v
                },
            },
            size,
        ))
    }
}

pub struct BlockInfo {
    pub name: String,
    pub backing_index: u32,
    pub path: String,
    pub read_requests: u64,
    pub read_bytes: u64,
    pub read_time: u64,
    pub write_requests: u64,
    pub write_bytes: u64,
    pub write_time: u64,
    pub flush_requests: u64,
    pub flush_time: u64,
    pub errors: u64,

    /// Logical size in bytes of the image (how much storage the guest
    /// will see).
    pub capacity: u64,
    /// Host storage in bytes occupied by the image (such as highest
    /// allocated extent if there are no holes, similar to 'du').
    pub allocation: u64,
    /// Host physical size in bytes of the image container (last
    /// offset, similar to 'ls')
    pub physical: u64,
}

pub struct InterfaceStats {
    pub name: String,
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub rx_errs: u64,
    pub rx_drop: u64,
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub tx_errs: u64,
    pub tx_drop: u64,
}

pub trait Params {
    fn get_u32(&self, key: &str) -> Option<u32>;
    fn get_u64(&self, key: &str) -> Option<u64>;
    fn get_string(&self, key: &str) -> Option<String>;
}

impl Params for Vec<RemoteTypedParam> {
    fn get_u32(&self, key: &str) -> Option<u32> {
        self.iter().find(|p| p.field == key).map(|p| p.as_u32())
    }

    fn get_u64(&self, key: &str) -> Option<u64> {
        self.iter().find(|p| p.field == key).map(|p| p.as_u64())
    }

    fn get_string(&self, key: &str) -> Option<String> {
        self.iter().find(|p| p.field == key).map(|p| p.as_string())
    }
}

pub struct DomainStatsRecord {
    domain: Domain,
    params: Vec<RemoteTypedParam>,
}

impl DomainStatsRecord {
    pub fn domain(&self) -> &Domain {
        &self.domain
    }

    pub fn blocks(&self) -> Vec<BlockInfo> {
        let n = self.get_u32("block.count").unwrap_or_default();
        let mut infos = Vec::with_capacity(n as usize);
        for i in 0..n {
            infos.push(BlockInfo {
                name: self
                    .params
                    .get_string(&format!("block.{}.name", i))
                    .unwrap_or_default(),
                backing_index: self
                    .params
                    .get_u32(&format!("block.{}.backingIndex", i))
                    .unwrap_or_default(),
                path: self
                    .params
                    .get_string(&format!("block.{}.path", i))
                    .unwrap_or_default(),
                read_requests: self
                    .params
                    .get_u64(&format!("block.{}.rd.reqs", i))
                    .unwrap_or_default(),
                read_bytes: self
                    .params
                    .get_u64(&format!("block.{}.rd.bytes", i))
                    .unwrap_or_default(),
                read_time: self
                    .params
                    .get_u64(&format!("block.{}.rd.times", i))
                    .unwrap_or_default(),
                write_requests: self
                    .params
                    .get_u64(&format!("block.{}.wr.reqs", i))
                    .unwrap_or_default(),
                write_bytes: self
                    .params
                    .get_u64(&format!("block.{}.wr.bytes", i))
                    .unwrap_or_default(),
                write_time: self
                    .params
                    .get_u64(&format!("block.{}.wr.times", i))
                    .unwrap_or_default(),
                flush_requests: self
                    .params
                    .get_u64(&format!("block.{}.fl.reqs", i))
                    .unwrap_or_default(),
                flush_time: self
                    .params
                    .get_u64(&format!("block.{}.fl.times", i))
                    .unwrap_or_default(),
                errors: self
                    .params
                    .get_u64(&format!("block.{}.errors", i))
                    .unwrap_or_default(),
                capacity: self
                    .params
                    .get_u64(&format!("block.{}.capacity", i))
                    .unwrap_or_default(),
                allocation: self
                    .params
                    .get_u64(&format!("block.{}.allocation", i))
                    .unwrap_or_default(),
                physical: self
                    .params
                    .get_u64(&format!("block.{}.physical", i))
                    .unwrap_or_default(),
            });
        }

        infos
    }

    pub fn networks(&self) -> Vec<InterfaceStats> {
        let n = self.get_u32("net.count").unwrap_or_default();
        let mut stats = Vec::with_capacity(n as usize);
        for i in 0..n {
            stats.push(InterfaceStats {
                name: self
                    .params
                    .get_string(format!("net.{}.name", i).as_str())
                    .unwrap_or_default(),
                rx_bytes: self
                    .params
                    .get_u64(format!("net.{}.rx.bytes", i).as_str())
                    .unwrap_or_default(),
                rx_packets: self
                    .params
                    .get_u64(format!("net.{}.rx.pkts", i).as_str())
                    .unwrap_or_default(),
                rx_errs: self
                    .params
                    .get_u64(format!("net.{}.rx.errs", i).as_str())
                    .unwrap_or_default(),
                rx_drop: self
                    .params
                    .get_u64(format!("net.{}.rx.drop", i).as_str())
                    .unwrap_or_default(),
                tx_bytes: self
                    .params
                    .get_u64(format!("net.{}.tx.bytes", i).as_str())
                    .unwrap_or_default(),
                tx_packets: self
                    .params
                    .get_u64(format!("net.{}.tx.pkets", i).as_str())
                    .unwrap_or_default(),
                tx_errs: self
                    .params
                    .get_u64(format!("net.{}.tx.errs", i).as_str())
                    .unwrap_or_default(),
                tx_drop: self
                    .params
                    .get_u64(format!("net.{}.tx.drop", i).as_str())
                    .unwrap_or_default(),
            });
        }

        stats
    }

    pub fn vcpu_delay_and_wait(&self, vcpu: u32) -> (u64, u64) {
        let delay = self
            .get_u64(format!("vcpu.{}.delay", vcpu).as_str())
            .unwrap_or_default();
        let wait = self
            .get_u64(format!("vcpu.{}.wait", vcpu).as_str())
            .unwrap_or_default();
        (delay, wait)
    }

    fn get_u32(&self, key: &str) -> Option<u32> {
        self.params
            .iter()
            .find(|p| p.field == key)
            .map(|p| match p.value {
                RemoteTypedParamValue::Const2(v) => v,
                _ => unreachable!(),
            })
    }

    fn get_u64(&self, key: &str) -> Option<u64> {
        self.params
            .iter()
            .find(|p| p.field == key)
            .map(|p| match p.value {
                RemoteTypedParamValue::Const4(v) => v,
                _ => unreachable!(),
            })
    }
}

impl<R: Read> Unpack<R> for DomainStatsRecord {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let (domain, s1) = Unpack::unpack(r)?;
        let (params, s2) = unpack_flex(r, REMOTE_CONNECT_GET_ALL_DOMAIN_STATS_MAX)?;

        Ok((Self { domain, params }, s1 + s2))
    }
}

pub struct GetAllDomainStatsRequest {
    pub domains: Vec<Domain>,
    pub stats: u32,
    pub flags: u32,
}

impl_procedure!(
    GetAllDomainStatsRequest,
    REMOTE_PROC_CONNECT_GET_ALL_DOMAIN_STATS
);

impl<W: Write> Pack<W> for GetAllDomainStatsRequest {
    fn pack(&self, w: &mut W) -> Result<usize> {
        let sz = pack_flex(&self.domains, Some(REMOTE_DOMAIN_LIST_MAX), w)?
            + self.stats.pack(w)?
            + self.flags.pack(w)?;

        Ok(sz)
    }
}

pub struct GetAllDomainStatsResponse {
    pub stats: Vec<DomainStatsRecord>,
}

impl<R: Read> Unpack<R> for GetAllDomainStatsResponse {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let (stats, sz) = unpack_flex(r, REMOTE_DOMAIN_LIST_MAX)?;

        Ok((Self { stats }, sz))
    }
}
