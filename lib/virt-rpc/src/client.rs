use crate::request;
use crate::request::{
    GetLibVersionRequest, GetLibVersionResponse, LibvirtMessage,
    REMOTE_PROC_CONNECT_GET_LIB_VERSION,
};
use crate::Error;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use bytes::{BufMut, BytesMut};
use tokio::net::UnixStream;
use xdr_codec::Pack;

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

    pub async fn version(&self) -> Result<(u32, u32, u32), Error> {
        let req = self.make_request(
            REMOTE_PROC_CONNECT_GET_LIB_VERSION,
            GetLibVersionRequest::new(),
        );



        todo!()
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

    async fn do_request<R, P>(&self) -> Result<P, Error>
    where
        R: ,
    {
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
            payload
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
        let stream = UnixStream::connect(path).await.unwrap();

        let f = stream.writable().await;
    }
}
