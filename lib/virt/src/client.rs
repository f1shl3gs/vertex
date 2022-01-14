use crate::request;
use crate::request::remote_procedure::{
    REMOTE_PROC_CONNECT_GET_ALL_DOMAIN_STATS, REMOTE_PROC_CONNECT_GET_VERSION,
    REMOTE_PROC_CONNECT_LIST_STORAGE_POOLS, REMOTE_PROC_DOMAIN_GET_BLOCK_IO_TUNE,
    REMOTE_PROC_DOMAIN_GET_INFO, REMOTE_PROC_DOMAIN_GET_VCPUS, REMOTE_PROC_DOMAIN_GET_XML_DESC,
    REMOTE_PROC_DOMAIN_MEMORY_STATS, REMOTE_PROC_STORAGE_POOL_GET_INFO,
};
use crate::request::{
    remote_procedure::{
        REMOTE_PROC_AUTH_LIST, REMOTE_PROC_CONNECT_GET_LIB_VERSION, REMOTE_PROC_CONNECT_OPEN,
    },
    virNetMessageError, virNetMessageHeader, virNetMessageStatus, AuthListRequest,
    AuthListResponse, BlockIoTuneParameters, ConnectOpenRequest, Domain,
    DomainGetBlockIoTuneRequest, DomainGetBlockIoTuneResponse, DomainGetInfoRequest,
    DomainGetInfoResponse, DomainGetVcpusRequest, DomainGetVcpusResponse, DomainGetXmlDescRequest,
    DomainGetXmlDescResponse, DomainInfo, DomainMemoryStatsRequest, DomainMemoryStatsResponse,
    DomainStatsRecord, GetAllDomainStatsRequest, GetAllDomainStatsResponse, GetLibVersionRequest,
    GetLibVersionResponse, GetVersionRequest, GetVersionResponse, LibvirtMessage,
    ListAllStoragePoolsRequest, ListAllStoragePoolsResponse, MemoryStats,
    StoragePoolGetInfoRequest, StoragePoolGetInfoResponse, VcpuInfo,
};
use crate::Error;
use std::io;
use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use xdr_codec::Unpack;

#[derive(Debug)]
pub struct StoragePoolInfo {
    pub name: String,

    /// A `StoragePoolState` flags
    pub state: u32,
    /// Logical size bytes.
    pub capacity: u64,
    /// Current allocation bytes.
    pub allocation: u64,
    /// Remaining free space bytes.
    pub available: u64,
}

pub struct Client {
    serial: AtomicU32,
    stream: UnixStream,
}

impl Client {
    pub async fn connect(path: impl AsRef<Path>) -> io::Result<Self> {
        let stream = UnixStream::connect(path).await?;
        let serial = AtomicU32::new(0);
        Ok(Self { serial, stream })
    }

    pub async fn version(&mut self) -> Result<String, Error> {
        let req = self.make_request(
            REMOTE_PROC_CONNECT_GET_LIB_VERSION,
            GetLibVersionRequest::new(),
        );

        let pkt: GetLibVersionResponse = self.do_request(req).await?;

        Ok(pkt.version())
    }

    pub async fn hyper_version(&mut self) -> Result<String, Error> {
        let req = self.make_request(REMOTE_PROC_CONNECT_GET_VERSION, GetVersionRequest::new());

        let resp: GetVersionResponse = self.do_request(req).await?;

        Ok(resp.version())
    }

    pub async fn auth(&mut self) -> Result<AuthListResponse, Error> {
        let req = self.make_request(REMOTE_PROC_AUTH_LIST, AuthListRequest::new());

        let pkt: AuthListResponse = self.do_request(req).await?;
        Ok(pkt)
    }

    pub async fn open(&mut self) -> Result<(), Error> {
        let req = self.make_request(REMOTE_PROC_CONNECT_OPEN, ConnectOpenRequest::new());
        self.do_request(req).await?;
        Ok(())
    }

    pub async fn domain_xml(&mut self, dom: &Domain) -> Result<String, Error> {
        let req = self.make_request(
            REMOTE_PROC_DOMAIN_GET_XML_DESC,
            DomainGetXmlDescRequest::new(dom.underlying()),
        );
        let resp: DomainGetXmlDescResponse = self.do_request(req).await?;
        Ok(resp.xml())
    }

    // Example of params
    //
    // state.state
    // state.reason
    // cpu.time
    // cpu.user
    // cpu.system
    // cpu.cache.monitor.count
    // cpu.haltpoll.success.time
    // cpu.haltpoll.fail.time
    // balloon.current
    // balloon.maximum
    // balloon.last-update
    // balloon.rss
    // vcpu.current
    // vcpu.maximum
    // vcpu.0.state
    // vcpu.0.time
    // vcpu.0.wait
    // vcpu.0.delay
    // vcpu.1.state
    // vcpu.1.time
    // vcpu.1.wait
    // vcpu.1.delay
    // net.count
    // net.0.name
    // net.0.rx.bytes
    // net.0.rx.pkts
    // net.0.rx.errs
    // net.0.rx.drop
    // net.0.tx.bytes
    // net.0.tx.pkts
    // net.0.tx.errs
    // net.0.tx.drop
    // net.1.name
    // net.1.rx.bytes
    // net.1.rx.pkts
    // net.1.rx.errs
    // net.1.rx.drop
    // net.1.tx.bytes
    // net.1.tx.pkts
    // net.1.tx.errs
    // net.1.tx.drop
    // block.count
    // block.0.name
    // block.0.path
    // block.0.backingIndex
    // block.0.rd.reqs
    // block.0.rd.bytes
    // block.0.rd.times
    // block.0.wr.reqs
    // block.0.wr.bytes
    // block.0.wr.times
    // block.0.fl.reqs
    // block.0.fl.times
    // block.0.allocation
    // block.0.capacity
    // block.0.physical
    // block.1.name
    // block.1.path
    // block.1.backingIndex
    // block.1.rd.reqs
    // block.1.rd.bytes
    // block.1.rd.times
    // block.1.wr.reqs
    // block.1.wr.bytes
    // block.1.wr.times
    // block.1.fl.reqs
    // block.1.fl.times
    // block.1.allocation
    // block.1.capacity
    // block.1.physical
    pub async fn get_all_domain_stats(&mut self) -> Result<Vec<DomainStatsRecord>, Error> {
        let req = self.make_request(
            REMOTE_PROC_CONNECT_GET_ALL_DOMAIN_STATS,
            GetAllDomainStatsRequest::new(127, 80),
        );

        let resp: GetAllDomainStatsResponse = self.do_request(req).await?;
        Ok(resp.stats())
    }

    pub async fn get_domain_info(&mut self, dom: &Domain) -> Result<DomainInfo, Error> {
        let req = self.make_request(
            REMOTE_PROC_DOMAIN_GET_INFO,
            DomainGetInfoRequest::new(dom.underlying()),
        );

        let resp: DomainGetInfoResponse = self.do_request(req).await?;
        Ok(resp.into())
    }

    pub async fn get_domain_vcpus(
        &mut self,
        dom: &Domain,
        maxinfo: i32,
    ) -> Result<Vec<VcpuInfo>, Error> {
        let req = self.make_request(
            REMOTE_PROC_DOMAIN_GET_VCPUS,
            DomainGetVcpusRequest::new(dom.underlying(), maxinfo),
        );

        let resp: DomainGetVcpusResponse = self.do_request(req).await?;
        Ok(resp.into())
    }

    pub async fn domain_memory_stats(
        &mut self,
        dom: &Domain,
        maxinfo: u32,
        flags: u32,
    ) -> Result<MemoryStats, Error> {
        let req = self.make_request(
            REMOTE_PROC_DOMAIN_MEMORY_STATS,
            DomainMemoryStatsRequest::new(dom.underlying(), maxinfo, flags),
        );

        let resp: DomainMemoryStatsResponse = self.do_request(req).await?;
        Ok(resp.into())
    }

    pub async fn block_io_tune(
        &mut self,
        dom: &Domain,
        disk: &str,
    ) -> Result<BlockIoTuneParameters, Error> {
        // first call to get nparams
        let req = self.make_request(
            REMOTE_PROC_DOMAIN_GET_BLOCK_IO_TUNE,
            DomainGetBlockIoTuneRequest::new(
                dom.underlying(),
                Some(request::generated::remote_nonnull_string(disk.to_string())),
                0,
            ),
        );

        let resp: DomainGetBlockIoTuneResponse = self.do_request(req).await?;
        let nparams = resp.nparams();

        // second call get all params kvs
        let req = self.make_request(
            REMOTE_PROC_DOMAIN_GET_BLOCK_IO_TUNE,
            DomainGetBlockIoTuneRequest::new(
                dom.underlying(),
                Some(request::generated::remote_nonnull_string(disk.to_string())),
                nparams,
            ),
        );

        let resp: DomainGetBlockIoTuneResponse = self.do_request(req).await?;
        Ok(resp.into())
    }

    pub async fn storage_pools(&mut self) -> Result<Vec<StoragePoolInfo>, Error> {
        let req = self.make_request(
            REMOTE_PROC_CONNECT_LIST_STORAGE_POOLS,
            ListAllStoragePoolsRequest::new(2), // 2 for active pools
        );
        let resp: ListAllStoragePoolsResponse = self.do_request(req).await?;
        let pools = resp.pools();
        let mut infos = Vec::with_capacity(pools.len());

        for pool in pools {
            let name = pool.name.0.clone();
            let req = self.make_request(
                REMOTE_PROC_STORAGE_POOL_GET_INFO,
                StoragePoolGetInfoRequest::new(pool),
            );
            let resp: StoragePoolGetInfoResponse = self.do_request(req).await?;
            infos.push(StoragePoolInfo {
                name,
                state: resp.0.state as u32,
                capacity: resp.0.capacity,
                allocation: resp.0.allocation,
                available: resp.0.available,
            })
        }

        Ok(infos)
    }

    fn serial(&self) -> u32 {
        self.serial.fetch_add(1, Ordering::Relaxed)
    }

    fn make_request<T>(
        &self,
        procedure: request::remote_procedure,
        payload: T,
    ) -> request::LibvirtMessage<T> {
        let serial = self.serial();
        LibvirtMessage {
            header: request::virNetMessageHeader {
                serial,
                proc_: procedure as i32,
                ..Default::default()
            },
            payload,
        }
    }

    async fn do_request<R, P>(&mut self, req: R) -> Result<P, Error>
    where
        R: xdr_codec::Pack<Cursor<Vec<u8>>>,
        P: xdr_codec::Unpack<Cursor<Vec<u8>>>,
    {
        let (size, buf) = {
            let buf = Vec::new();
            let mut c = Cursor::new(buf);
            let size = req.pack(&mut c)?;
            (size, c.into_inner())
        };

        let len = size + 4;
        self.stream.write_u32(len as u32).await?;
        self.stream.write_all(&buf[0..size]).await?;

        // read response
        let len = self.stream.read_u32().await?;
        let len = len - 4; // skip len

        // read whole packet
        let mut buf = vec![0; len as usize];
        self.stream.read_exact(&mut buf[0..len as usize]).await?;
        let mut cur = Cursor::new(buf);

        let (header, _) = virNetMessageHeader::unpack(&mut cur)?;
        if header.status == virNetMessageStatus::VIR_NET_OK {
            let (pkt, _) = P::unpack(&mut cur)?;
            return Ok(pkt);
        }

        let (err, _) = virNetMessageError::unpack(&mut cur)?;
        Err(Error::from(err))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::print_stdout)] // tests

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn connect() {
        let path = "/run/libvirt/libvirt-sock-ro";
        let mut cli = Client::connect(path).await.unwrap();
        // let auth = cli.auth().await.unwrap();
        cli.open().await.unwrap();
        let version = cli.version().await.unwrap();
        println!("libvirt: {}", version);
        let version = cli.hyper_version().await.unwrap();
        println!("hyper: {}", version);

        let stats = cli.get_all_domain_stats().await.unwrap();
        for s in stats {
            let dom = s.domain();
            let info = cli.get_domain_info(&dom).await.unwrap();
            println!("dom: {}", dom.name());

            // vcpu
            let vcpus = cli
                .get_domain_vcpus(&dom, info.nr_virt_cpu as i32)
                .await
                .unwrap();
            for vcpu in vcpus {
                println!("vcpu:{} -> {}", vcpu.number, vcpu.cpu);
                let (delay, wait) = s.vcpu_delay_and_wait(vcpu.number);
                println!("vcpu:{}  delay {} wait {}", vcpu.number, delay, wait);
            }

            let blocks = s.blocks();
            for block in blocks {
                println!("block: {}", block.name);
                let params = cli.block_io_tune(&dom, &block.name).await.unwrap();
                println!("{:#?}", params);
            }
        }

        let pools = cli.storage_pools().await.unwrap();
        for pool in pools {
            println!("pool: {}", pool.name);
        }
    }
}
