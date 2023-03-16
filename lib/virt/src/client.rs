use std::io;
use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::error::Error;
use crate::protocol::{
    AuthListRequest, AuthListResponse, AuthType, BlockIoTuneParameters,
    ConnectListStoragePoolsRequest, ConnectListStoragePoolsResponse, Domain,
    DomainGetXmlDescRequest, DomainGetXmlDescResponse, DomainInfo, DomainMemoryStatsRequest,
    DomainMemoryStatsResponse, DomainStatsRecord, GetAllDomainStatsRequest,
    GetAllDomainStatsResponse, GetDomainBlockIoTuneRequest, GetDomainBlockIoTuneResponse,
    GetDomainInfoRequest, GetDomainInfoResponse, GetDomainVcpusRequest, GetDomainVcpusResponse,
    GetLibVersionRequest, GetStoragePoolInfoRequest, GetStoragePoolInfoResponse, GetVersionRequest,
    GetVersionResponse, MemoryStats, OpenRequest, OpenResponse, Procedure, VcpuInfo,
};
use crate::protocol::{
    GetLibVersionResponse, MessageError as ProtocolError, MessageHeader, Pack, Unpack,
};

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
        let serial = AtomicU32::new(1);
        Ok(Self { serial, stream })
    }

    pub async fn version(&mut self) -> Result<String, Error> {
        let req = GetLibVersionRequest {};

        let resp: GetLibVersionResponse = self.send(req).await?;

        Ok(resp.version())
    }

    pub async fn hyper_version(&mut self) -> Result<String, Error> {
        let req = GetVersionRequest {};
        let resp: GetVersionResponse = self.send(req).await?;
        Ok(resp.version())
    }

    pub async fn auth(&mut self) -> Result<Vec<AuthType>, Error> {
        let req = AuthListRequest {};
        let resp: AuthListResponse = self.send(req).await?;
        Ok(resp.types)
    }

    pub async fn open(&mut self) -> Result<(), Error> {
        let req = OpenRequest::default();
        let _resp: OpenResponse = self.send(req).await?;
        Ok(())
    }

    pub async fn domain_xml(&mut self, domain: &Domain) -> Result<String, Error> {
        let req = DomainGetXmlDescRequest { domain, flags: 0 };
        let resp: DomainGetXmlDescResponse = self.send(req).await?;
        Ok(resp.data)
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
        let req = GetAllDomainStatsRequest {
            domains: vec![],
            stats: 128,
            flags: 80,
        };

        let resp: GetAllDomainStatsResponse = self.send(req).await?;
        Ok(resp.stats)
    }

    pub async fn get_domain_info(&mut self, domain: &Domain) -> Result<DomainInfo, Error> {
        let req = GetDomainInfoRequest { domain };

        let resp: GetDomainInfoResponse = self.send(req).await?;
        Ok(resp.info)
    }

    pub async fn get_domain_vcpus(
        &mut self,
        domain: &Domain,
        max_info: i32,
    ) -> Result<Vec<VcpuInfo>, Error> {
        let req = GetDomainVcpusRequest {
            domain,
            max_info,
            map_len: 0,
        };

        let resp: GetDomainVcpusResponse = self.send(req).await?;
        Ok(resp.infos)
    }

    pub async fn domain_memory_stats(
        &mut self,
        domain: &Domain,
        max_stats: u32,
        flags: u32,
    ) -> Result<MemoryStats, Error> {
        let req = DomainMemoryStatsRequest {
            domain,
            max_stats,
            flags,
        };

        let resp: DomainMemoryStatsResponse = self.send(req).await?;
        Ok(resp.stats)
    }

    pub async fn block_io_tune(
        &mut self,
        domain: &Domain,
        disk: &str,
    ) -> Result<BlockIoTuneParameters, Error> {
        // first call to get nparams
        let req = GetDomainBlockIoTuneRequest {
            domain,
            disk,
            nparams: 0,
            flags: 0,
        };

        let resp: GetDomainBlockIoTuneResponse = self.send(req).await?;

        // second call get all params kvs
        let req = GetDomainBlockIoTuneRequest {
            domain,
            disk,
            nparams: resp.nparams,
            flags: 0,
        };

        let resp: GetDomainBlockIoTuneResponse = self.send(req).await?;
        Ok(resp.block_io_tune_parameters())
    }

    pub async fn storage_pools(&mut self) -> Result<Vec<StoragePoolInfo>, Error> {
        /*        let req = self.make_request(
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

        Ok(infos)*/

        let resp: ConnectListStoragePoolsResponse = self
            .send(ConnectListStoragePoolsRequest { maxnames: 2 })
            .await?;
        let mut infos = Vec::with_capacity(resp.pools.len());

        for pool in resp.pools {
            let req = GetStoragePoolInfoRequest { pool: &pool };

            let resp: GetStoragePoolInfoResponse = self.send(req).await?;
            infos.push(StoragePoolInfo {
                name: pool.name,
                state: resp.state as u32,
                capacity: resp.capacity,
                allocation: resp.allocation,
                available: resp.available,
            });
        }

        Ok(infos)
    }

    fn serial_id(&self) -> u32 {
        self.serial.fetch_add(1, Ordering::SeqCst)
    }

    async fn send<R, P>(&mut self, req: R) -> Result<P, Error>
    where
        R: Pack<Cursor<Vec<u8>>> + Procedure,
        P: Unpack<Cursor<Vec<u8>>>,
    {
        let (size, buf) = {
            let buf = Vec::new();
            let mut c = Cursor::new(buf);
            let header = MessageHeader {
                serial: self.serial_id(),
                procedure: R::procedure(),
                ..Default::default()
            };

            // serialize header
            header.pack(&mut c)?;
            // serialize body
            req.pack(&mut c)?;

            let buf = c.into_inner();
            (buf.len(), buf)
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

        let (header, _sz) = MessageHeader::unpack(&mut cur)?;
        if header.success() {
            let (pkt, _) = P::unpack(&mut cur)?;
            return Ok(pkt);
        }

        let (err, _) = ProtocolError::unpack(&mut cur)?;
        Err(Error::from(err))
    }
}
