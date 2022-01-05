use crate::request;
use crate::request::{
    remote_procedure::{
        REMOTE_PROC_AUTH_LIST, REMOTE_PROC_CONNECT_GET_LIB_VERSION, REMOTE_PROC_CONNECT_OPEN, REMOTE_PROC_CONNECT_LIST_DOMAINS
    },
    virNetMessageError, virNetMessageHeader, virNetMessageStatus, AuthListRequest,
    AuthListResponse, ConnectOpenRequest, ConnectOpenResponse, GetLibVersionRequest,
    GetLibVersionResponse, LibvirtMessage, ListAllDomainsFlags, ListAllDomainsRequest,
    ListAllDomainsResponse,
};
use crate::Error;
use bytes::{BufMut, BytesMut};
use std::io;
use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use xdr_codec::{Pack, Unpack};

pub struct LibvirtRequest {
    header: request::virNetMessageHeader,
    payload: BytesMut,
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

    pub async fn version(&mut self) -> Result<(u32, u32, u32), Error> {
        let req = self.make_request(
            REMOTE_PROC_CONNECT_GET_LIB_VERSION,
            GetLibVersionRequest::new(),
        );

        let pkt: GetLibVersionResponse = self.do_request(req).await?;

        Ok(pkt.version())
    }

    pub async fn auth(&mut self) -> Result<AuthListResponse, Error> {
        let req = self.make_request(REMOTE_PROC_AUTH_LIST, AuthListRequest::new());

        let pkt: AuthListResponse = self.do_request(req).await?;
        Ok(pkt)
    }

    pub async fn open(&mut self) -> Result<ConnectOpenResponse, Error> {
        let req = self.make_request(REMOTE_PROC_CONNECT_OPEN, ConnectOpenRequest::new());
        self.do_request(req).await
    }

    pub async fn list_all_domains(
        &mut self,
        flags: ListAllDomainsFlags,
    ) -> Result<ListAllDomainsResponse, Error> {
        let req = self.make_request(
            REMOTE_PROC_CONNECT_LIST_DOMAINS,
            ListAllDomainsRequest::new(flags),
        );
        self.do_request(req).await
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

    fn pack<P: Pack<bytes::buf::Writer<bytes::BytesMut>>>(
        procedure: request::remote_procedure,
        payload: P,
    ) -> Result<LibvirtRequest, xdr_codec::Error> {
        let payload = {
            let buf = BytesMut::with_capacity(4 * 1024);
            let mut writer = buf.writer();
            payload.pack(&mut writer)?;
            writer.into_inner()
        };

        Ok(LibvirtRequest {
            header: request::virNetMessageHeader {
                proc_: procedure as i32,
                ..Default::default()
            },
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::UnixStream;

    #[tokio::test]
    async fn connect() {
        let path = "/run/libvirt/libvirt-sock-ro";
        let mut cli = Client::connect(path).await.unwrap();
        // let auth = cli.auth().await.unwrap();
        let resp = cli.open().await.unwrap();
        let (major, minor, micro) = cli.version().await.unwrap();
        println!("{}.{}.{}", major, minor, micro);
        let resp = cli
            .list_all_domains(
                ListAllDomainsFlags::DOMAINS_ACTIVE | ListAllDomainsFlags::DOMAINS_INACTIVE,
            )
            .await
            .unwrap();
        let domains: Vec<request::Domain> = resp.into();
        for dom in domains {
            println!("{:#?}", dom)
        }
    }
}
