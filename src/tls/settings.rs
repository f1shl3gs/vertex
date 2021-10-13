use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    io,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::task::Poll;
use futures::future::BoxFuture;
use futures::{Future, FutureExt};
use snafu::ResultExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsStream};
use tokio_stream::Stream;
use crate::tls::{MaybeTLS, MaybeTLSStream, TLSError};
use crate::tls::incoming::MaybeTLSIncomingStream;
use super::{IncomingListener, TcpBind};

const PEM_START_MARKER: &str = "-----BEGIN ";

#[cfg(test)]
pub const TEST_PEM_CA_PATH: &str = "tests/data/Vector_CA.crt";
#[cfg(test)]
pub const TEST_PEM_CRT_PATH: &str = "tests/data/localhost.crt";
#[cfg(test)]
pub const TEST_PEM_KEY_PATH: &str = "tests/data/localhost.key";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TLSConfig {
    pub verify_certificate: Option<bool>,
    pub verify_hostname: Option<bool>,
    pub ca_file: Option<PathBuf>,
    pub crt_file: Option<PathBuf>,
    pub key_file: Option<PathBuf>,
    pub key_pass: Option<String>,
}

impl TLSConfig {
    #[cfg(test)]
    pub fn test_options() -> Self {
        Self {
            ca_file: Some(TEST_PEM_CA_PATH.into()),
            crt_file: Some(TEST_PEM_CRT_PATH.into()),
            key_file: Some(TEST_PEM_KEY_PATH.into()),
            ..Self::default()
        }
    }

    fn load_authorities(&self) {
        todo!()
    }
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

#[derive(Clone)]
pub struct IdentityStore(Vec<u8>, String);

/// Directly usable settings for TLS connectors
#[derive(Clone, Default)]
pub struct TLSSettings {
    verify_certificate: bool,
    verify_hostname: bool,
    // authorities: Vec<rustls::x>,
    identity: Option<IdentityStore>,
}

impl TLSSettings {
    /// Generate a filled out settings struct from the given optional
    /// option set, interpreted as client options. If `options` is
    /// `None`, the result is set to defaults(ie: empty)
    pub fn from_config(conf: &TLSConfig) -> Result<Self, TLSError> {
        // If this is for server warning should be print

        Ok(Self {
            verify_certificate: conf.verify_certificate.unwrap_or(false),
            verify_hostname: conf.verify_hostname.unwrap_or(false),
            identity: None,
        })
    }

    pub fn acceptor(&self) -> Result<TlsAcceptor, TLSError> {
        match self.identity {
            None => Err(TLSError::MissingRequiredIdentity),
            Some(_) => {
                let conf = rustls::ServerConfig::from();

                let mut acceptor = TlsAcceptor::from();
            }
        }
    }
}

impl<R, T> MaybeTLS<R, T> {
    // pub async fn bind(&self, addr: &SocketAddr) ->
}

pub type MaybeTLSSettings = MaybeTLS<(), TLSSettings>;

impl MaybeTLSSettings {
    ///
    pub fn from_config(conf: &Option<TLSConfig>) -> Result<Self, TLSError> {
        match conf {
            None => Ok(Self::Raw(())), // No config, no TLS settings
            Some(config) => {
                let tls = TLSSettings::from_config(config)?;
                Ok(Self::TLS(tls))
            }
        }
    }

    pub async fn bind(&self, addr: &SocketAddr) -> Result<MaybeTLSListener, TLSError> {
        let listener = TcpListener::bind(addr)
            .await
            .context(TcpBind)?;

        let acceptor = match self {
            Self::Raw(()) => None,
            Self::TLS(tls) => Some(tls.acceptor()?),
        };

        Ok(MaybeTLSListener {
            listener,
            acceptor,
        })
    }
}

pub struct MaybeTLSListener {
    listener: TcpListener,
    acceptor: Option<TlsAcceptor>,
}

impl From<TcpListener> for MaybeTLSListener {
    fn from(listener: TcpListener) -> Self {
        Self {
            listener,
            acceptor: None,
        }
    }
}

impl MaybeTLSListener {
    pub async fn accept(
        &mut self,
    ) -> Result<MaybeTLSIncomingStream<TcpStream>, TLSError> {
        self.listener
            .accept()
            .await
            .map(|(stream, addr)| {
                MaybeTLSIncomingStream::new(stream, addr, self.acceptor.clone())
            })
            .context(IncomingListener)
    }

    async fn into_accept(
        mut self,
    ) -> (Result<MaybeTLSIncomingStream<TcpStream>, TLSError>, Self) {
        (self.accept().await, self)
    }

    pub fn accept_stream(
        self,
    ) -> impl Stream<Item=Result<MaybeTLSIncomingStream<TcpStream>, TLSError>>
    {
        let mut accept = Box::pin(self.into_accept());
        futures::stream::poll_fn(move |cx| {
            match accept.as_mut().poll(cx) {
                Poll::Ready((item, this)) => {
                    accept.set(this.into_accept());
                    Poll::Ready(Some(item))
                }

                Poll::Pending => Poll::Pending
            }
        })
    }
}
