use serde::{Deserialize, Serialize};
use std::{
    io,
    fs,
    path::PathBuf,
    net::SocketAddr,
    sync::Arc,
    task::Poll,
};
use std::time::SystemTime;
use futures::{Future};
use rustls::{Certificate, ClientConfig, Error, RootCertStore, ServerName};
use rustls::client::ServerCertVerified;
use snafu::ResultExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor};
use tokio_stream::Stream;
use crate::tls::{MaybeTLS, TLSError, incoming::MaybeTLSIncomingStream};
use super::{IncomingListener, TcpBind, FileOpenFailed, ReadPemFailed};

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
    certs: Vec<rustls::Certificate>,
    keys: Vec<rustls::PrivateKey>,
}

impl TLSSettings {
    /// Generate a filled out settings struct from the given optional
    /// option set, interpreted as client options. If `options` is
    /// `None`, the result is set to defaults(ie: empty)
    pub fn from_config(conf: &Option<TLSConfig>) -> Result<Self, TLSError> {
        // If this is for server warning should be print

        let default = &TLSConfig::default();
        let conf = conf.as_ref().unwrap_or(&default);

        // Load public certificate
        let certs = {
            match &conf.crt_file {
                None => vec![],
                Some(filename) => {
                    let note: &'static str = "certificate";
                    let f = fs::File::open(filename)
                        .with_context(|| FileOpenFailed { note, filename })?;
                    let mut reader = io::BufReader::new(f);

                    let certs = rustls_pemfile::certs(&mut reader)
                        .with_context(|| ReadPemFailed { filename })?;

                    certs.into_iter()
                        .map(rustls::Certificate)
                        .collect::<Vec<_>>()
                }
            }
        };

        // Load private key
        let keys = match &conf.crt_file {
            None => vec![rustls::PrivateKey(vec![])],
            Some(filename) => {
                let note: &'static str = "private key";
                let f = fs::File::open(&filename)
                    .with_context(|| FileOpenFailed { note, filename })?;
                let mut reader = io::BufReader::new(f);

                // Load and retun a single private key
                match rustls_pemfile::rsa_private_keys(&mut reader) {
                    Ok(keys) => {
                        keys.iter()
                            .map(|v| rustls::PrivateKey(v.clone()))
                            .collect::<Vec<_>>()
                    }
                    _ => return Err(TLSError::PrivateKeyParseError {
                        filename: filename.clone()
                    })
                }
            }
        };

        Ok(Self {
            verify_certificate: conf.verify_certificate.unwrap_or(false),
            verify_hostname: conf.verify_hostname.unwrap_or(false),
            certs,
            keys,
            // server_config,
        })
    }

    pub fn acceptor(&self) -> Result<TlsAcceptor, TLSError> {
        let conf = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(self.certs.clone(), self.keys[0].clone())
            .expect("Bad certificate/key");

        let acceptor = TlsAcceptor::from(Arc::new(conf));
        Ok(acceptor)
    }
}

impl From<TLSSettings> for MaybeTLSSettings {
    fn from(tls: TLSSettings) -> Self {
        Self::Tls(tls)
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
                let tls = TLSSettings::from_config(&Some(config.clone()))?;
                Ok(Self::Tls(tls))
            }
        }
    }

    pub async fn bind(&self, addr: &SocketAddr) -> Result<MaybeTLSListener, TLSError> {
        let listener = TcpListener::bind(addr)
            .await
            .context(TcpBind)?;

        let acceptor = match self {
            Self::Raw(()) => None,
            Self::Tls(tls) => Some(tls.acceptor()?),
        };

        Ok(MaybeTLSListener {
            listener,
            acceptor,
        })
    }

    pub fn client_config(&self) -> Result<ClientConfig, TLSError> {
        let root_store = RootCertStore::empty();
        // TODO: handle root_store properly

        let mut conf = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();

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
        _end_entity: &Certificate,
        _intermediates: &[Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item=&[u8]>,
        _ocsp_response: &[u8],
        _now: SystemTime,
    ) -> Result<ServerCertVerified, Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
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

        let _settings = TLSSettings::from_config(&Some(cfg))
            .unwrap();
    }
}
