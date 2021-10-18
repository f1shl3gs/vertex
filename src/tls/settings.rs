use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    io,
    fs,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::task::Poll;
use futures::future::BoxFuture;
use futures::{Future, FutureExt};
use rustls::ClientConfig;
use snafu::ResultExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsStream};
use tokio_stream::Stream;
use rustls::internal::pemfile;
use crate::tls::{MaybeTLS, MaybeTLSStream, TLSError};
use crate::tls::incoming::MaybeTLSIncomingStream;
use super::{IncomingListener, TcpBind, FileOpenFailed, CertificateParseError};

const PEM_START_MARKER: &str = "-----BEGIN ";

#[cfg(test)]
pub const TEST_PEM_CA_PATH: &str = "testdata/tls/Vertex_CA.crt";
#[cfg(test)]
pub const TEST_PEM_CRT_PATH: &str = "testdata/tls/localhost.crt";
#[cfg(test)]
pub const TEST_PEM_KEY_PATH: &str = "testdata/tls/localhost.key";

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
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

#[derive(Clone)]
pub struct IdentityStore(Vec<u8>, String);

/// Directly usable settings for TLS connectors
#[derive(Clone)]
pub struct TLSSettings {
    verify_certificate: bool,
    verify_hostname: bool,
    server_config: rustls::ServerConfig,
}

impl TLSSettings {
    /// Generate a filled out settings struct from the given optional
    /// option set, interpreted as client options. If `options` is
    /// `None`, the result is set to defaults(ie: empty)
    pub fn from_config(conf: &TLSConfig) -> Result<Self, TLSError> {
        // If this is for server warning should be print

        // Load public certificate
        let certs = {
            let note: &'static str = "certificate";
            let filename = &conf.crt_file
                .clone()
                .unwrap();
            let f = fs::File::open(filename)
                .with_context(|| FileOpenFailed { note, filename })?;
            let mut reader = io::BufReader::new(f);
            // TODO: handle the error properly
            pemfile::certs(&mut reader).unwrap()
        };

        // Load private key
        let key = {
            let note: &'static str = "private key";
            let filename = &conf.key_file
                .clone()
                .unwrap();

            let f = fs::File::open(&filename)
                .with_context(|| FileOpenFailed { note, filename })?;
            let mut reader = io::BufReader::new(f);

            // Load and retun a single private key
            match pemfile::rsa_private_keys(&mut reader) {
                Ok(keys) => keys,
                _ => return Err(TLSError::PrivateKeyParseError {
                    filename: filename.clone()
                })
            }
        };

        // Do not use client certificate authentication
        let mut server_config = rustls::ServerConfig::new(rustls::NoClientAuth::new());
        // Select a certificate to use
        server_config.set_single_cert(certs, key[0].clone());
        server_config.set_protocols(&[b"h2".to_vec(), b"http/1.1".to_vec()]);

        Ok(Self {
            verify_certificate: conf.verify_certificate.unwrap_or(false),
            verify_hostname: conf.verify_hostname.unwrap_or(false),
            server_config,
        })
    }

    pub fn acceptor(&self) -> Result<TlsAcceptor, TLSError> {
        /*match self.identity {
            None => Err(TLSError::MissingRequiredIdentity),
            Some(_) => {
                let acceptor = TlsAcceptor::from(Arc::new(self.server_config));
                Ok(acceptor)
            }
        }*/

        let acceptor = TlsAcceptor::from(Arc::new(self.server_config.clone()));
        Ok(acceptor)
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

    pub fn client_config(&self) -> Result<ClientConfig, TLSError> {
        let mut conf = rustls::ClientConfig::new();

        if let Some(tls) = self.tls() {
            if tls.verify_certificate {
                conf.dangerous()
                    .set_certificate_verifier(Arc::new(NoCertificateVerification {}));
            }


        }

        Ok(conf)
    }
}

struct NoCertificateVerification {}

impl rustls::client::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item=&[u8]>,
        _ocsp: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::ServerCertVerified, rustls::Error> {
        Ok(rustls::ServerCertVerified::assertion())
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

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PEM_CRT_BYTES: &[u8] = include_bytes!("../../testdata/tls/localhost.crt");
    const TEST_PEM_KEY_BYTES: &[u8] = include_bytes!("../../testdata/tls/localhost.key");

    #[test]
    fn from_options_pem() {
        let cfg = TLSConfig {
            verify_certificate: None,
            verify_hostname: None,
            ca_file: None,
            crt_file: Some(TEST_PEM_CRT_PATH.into()),
            key_file: Some(TEST_PEM_KEY_PATH.into()),
            key_pass: None,
        };

        let _settings = TLSSettings::from_config(&cfg)
            .unwrap();
    }
}
