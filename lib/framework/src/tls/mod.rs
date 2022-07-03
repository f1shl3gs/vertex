use std::{fmt::Debug, net::SocketAddr, path::PathBuf};

use openssl::{
    error::ErrorStack,
    ssl::{ConnectConfiguration, SslConnector, SslConnectorBuilder, SslMethod},
};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_openssl::SslStream;

use crate::tcp::{self, TcpKeepaliveConfig};

#[cfg(feature = "tls-utils")]
mod incoming;
mod maybe_tls;
mod outgoing;
mod settings;

#[cfg(all(feature = "tls-utils", feature = "listenfd"))]
pub use incoming::{MaybeTlsIncomingStream, MaybeTlsListener};
pub use maybe_tls::MaybeTls;
pub use settings::{MaybeTlsSettings, TlsConfig, TlsOptions, TlsSettings};
#[cfg(any(test, feature = "test-util"))]
pub use settings::{TEST_PEM_CA_PATH, TEST_PEM_CRT_PATH, TEST_PEM_KEY_PATH};

pub type Result<T> = std::result::Result<T, TlsError>;

pub type MaybeTlsStream<S> = MaybeTls<S, SslStream<S>>;

#[derive(Debug, Error)]
pub enum TlsError {
    #[error("Could not open {note} file {filename:?}: {err}")]
    FileOpenFailed {
        note: &'static str,
        filename: PathBuf,
        err: std::io::Error,
    },
    #[error("Could not read {note} file {filename:?}: {err}")]
    FileReadFailed {
        note: &'static str,
        filename: PathBuf,
        err: std::io::Error,
    },
    #[error("Could not build TLS connector: {0}")]
    BuildConnector(ErrorStack),
    #[error("Could not set TCP TLS identity: {0}")]
    Identity(ErrorStack),
    #[error("Could not export identity to DER: {0}")]
    DerExport(ErrorStack),
    #[error("Identity certificate is missing a key")]
    MissingKey,
    #[error("Certificate file contains no certificates")]
    MissingCertificate,
    #[error("Could not parse certificate in {filename:?}: {err}")]
    CertificateParse { filename: PathBuf, err: ErrorStack },
    #[error("Must specify both TLS key_file and crt_file")]
    MissingCrtKeyFile,
    #[error("Could not parse X509 certificate in {filename:?}: {err}")]
    X509Parse { filename: PathBuf, err: ErrorStack },
    #[error("Could not parse private key in {filename:?}: {err}")]
    PrivateKeyParse { filename: PathBuf, err: ErrorStack },
    #[error("Could not build PKCS#12 archive for identity: {0}")]
    BuildPkcs12(ErrorStack),
    #[error("Could not parse identity in {filename:?}: {err}")]
    IdentityParse { filename: PathBuf, err: ErrorStack },
    #[error("TLS configuration requires a certificate when enabled")]
    MissingRequiredIdentity,
    #[error("TLS handshake failed: {0}")]
    Handshake(openssl::ssl::Error),
    #[error("Incoming listener failed: {0}")]
    IncomingListener(tokio::io::Error),
    #[error("Creating the TLS acceptor failed: {0}")]
    CreateAcceptor(ErrorStack),
    #[error("Error building SSL context: {0}")]
    SslBuild(ErrorStack),
    #[error("Error setting up the TLS certificate: {0}")]
    SetCertificate(ErrorStack),
    #[error("Error setting up the TLS private key: {0}")]
    SetPrivateKey(ErrorStack),
    #[error("Error setting up the TLS chain certificates: {0}")]
    AddExtraChainCert(ErrorStack),
    #[error("Error creating a certificate store: {0}")]
    NewStoreBuilder(ErrorStack),
    #[error("Error adding a certificate to a store: {0}")]
    AddCertToStore(ErrorStack),
    #[error("Error setting up the verification certificate: {0}")]
    SetVerifyCert(ErrorStack),
    #[error("PKCS#12 parse failed: {0}")]
    ParsePkcs12(ErrorStack),
    #[error("TCP bind failed: {0}")]
    TcpBind(tokio::io::Error),
    #[error("{0}")]
    Connect(tokio::io::Error),
    #[error("Could not get peer address: {0}")]
    PeerAddress(std::io::Error),
    #[error("Security Framework Error: {0}")]
    #[cfg(target_os = "macos")]
    SecurityFramework(security_framework::base::Error),
    #[error("Schannel Error: {0}")]
    #[cfg(windows)]
    Schannel(std::io::Error),
    #[cfg(any(windows, target_os = "macos"))]
    #[error("Unable to parse X509 from system cert: {0}")]
    X509SystemParseError(ErrorStack),
    #[error("Creating an empty CA stack failed")]
    NewCaStack(ErrorStack),
    #[error("Could not push intermediate certificate onto stack")]
    CaStackPush(ErrorStack),
}

impl MaybeTlsStream<TcpStream> {
    pub fn peer_addr(&self) -> std::result::Result<SocketAddr, std::io::Error> {
        match self {
            Self::Raw(raw) => raw.peer_addr(),
            Self::Tls(tls) => tls.get_ref().peer_addr(),
        }
    }

    pub fn set_keepalive(&mut self, keepalive: TcpKeepaliveConfig) -> std::io::Result<()> {
        let stream = match self {
            Self::Raw(raw) => raw,
            Self::Tls(tls) => tls.get_ref(),
        };

        if let Some(timeout) = keepalive.timeout {
            let config = socket2::TcpKeepalive::new().with_time(timeout);

            tcp::set_keepalive(stream, &config)?;
        }

        Ok(())
    }

    pub fn set_send_buffer_bytes(&mut self, bytes: usize) -> std::io::Result<()> {
        let stream = match self {
            Self::Raw(raw) => raw,
            Self::Tls(tls) => tls.get_ref(),
        };

        tcp::set_send_buffer_size(stream, bytes)
    }

    pub fn set_receive_buffer_bytes(&mut self, bytes: usize) -> std::io::Result<()> {
        let stream = match self {
            Self::Raw(raw) => raw,
            Self::Tls(tls) => tls.get_ref(),
        };

        tcp::set_receive_buffer_size(stream, bytes)
    }
}

pub(crate) fn tls_connector_builder(settings: &MaybeTlsSettings) -> Result<SslConnectorBuilder> {
    let mut builder = SslConnector::builder(SslMethod::tls()).map_err(TlsError::BuildConnector)?;
    if let Some(settings) = settings.tls() {
        settings.apply_context(&mut builder)?;
    }
    Ok(builder)
}

fn tls_connector(settings: &MaybeTlsSettings) -> Result<ConnectConfiguration> {
    let verify_hostname = settings
        .tls()
        .map(|settings| settings.verify_hostname)
        .unwrap_or(true);
    let configure = tls_connector_builder(settings)?
        .build()
        .configure()
        .map_err(TlsError::BuildConnector)?
        .verify_hostname(verify_hostname);
    Ok(configure)
}
