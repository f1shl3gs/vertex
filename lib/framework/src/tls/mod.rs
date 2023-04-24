mod incoming;
mod maybe_tls;
mod outgoing;
mod settings;

use std::{fmt::Debug, io, path::PathBuf};

#[cfg(feature = "listenfd")]
pub use incoming::{MaybeTlsIncomingStream, MaybeTlsListener};
pub use maybe_tls::MaybeTls;
pub use outgoing::MaybeTlsStream;
pub use settings::TlsConfig;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, TlsError>;

#[derive(Debug, Error)]
pub enum TlsError {
    #[error("Could not open {note} file {filename:?}: {err}")]
    FileOpenFailed {
        note: &'static str,
        filename: PathBuf,
        err: io::Error,
    },
    #[error("Could not read {note} file {filename:?}: {err}")]
    FileReadFailed {
        note: &'static str,
        filename: PathBuf,
        err: io::Error,
    },
    #[error("Could not build TLS connector: {0}")]
    BuildConnector(io::Error),
    #[error("Could not set TCP TLS identity: {0}")]
    Identity(io::Error),
    #[error("Could not export identity to DER: {0}")]
    DerExport(io::Error),
    #[error("Identity certificate is missing a key")]
    MissingKey,
    #[error("Certificate file contains no certificates")]
    MissingCertificate,
    #[error("Certificate and PrivateKey must be set")]
    MissingCertAndKey,
    #[error("Could not parse certificate in {filename:?}: {err}")]
    CertificateParse { filename: PathBuf, err: io::Error },
    #[error("Must specify both TLS key_file and crt_file")]
    MissingCrtKeyFile,
    #[error("Could not parse X509 certificate in {filename:?}: {err}")]
    X509Parse { filename: PathBuf, err: io::Error },
    #[error("Could not parse private key in {filename:?}: {err}")]
    PrivateKeyParse { filename: PathBuf, err: io::Error },
    #[error("Could not build PKCS#12 archive for identity: {0}")]
    BuildPkcs12(io::Error),
    #[error("Could not parse identity in {filename:?}: {err}")]
    IdentityParse { filename: PathBuf, err: io::Error },
    #[error("TLS configuration requires a certificate when enabled")]
    MissingRequiredIdentity,
    #[error("TLS handshake failed: {0}")]
    Handshake(io::Error),
    #[error("Incoming listener failed: {0}")]
    IncomingListener(io::Error),
    #[error("Invalid Server Name")]
    InvalidServerName,
    #[error("Creating the TLS acceptor failed: {0}")]
    CreateAcceptor(io::Error),
    #[error("Error building TLS config: {0}")]
    TlsBuild(rustls::Error),
    #[error("Error setting up the TLS certificate: {0}")]
    SetCertificate(io::Error),
    #[error("Error setting up the TLS private key: {0}")]
    SetPrivateKey(io::Error),
    #[error("Error setting up the TLS chain certificates: {0}")]
    AddExtraChainCert(io::Error),
    #[error("Error creating a certificate store: {0}")]
    NewStoreBuilder(io::Error),
    #[error("Error adding a certificate to a store: {0}")]
    AddCertToStore(rustls::Error),
    #[error("Error setting up the verification certificate: {0}")]
    SetVerifyCert(io::Error),
    #[error("PKCS#12 parse failed: {0}")]
    ParsePkcs12(io::Error),
    #[error("TCP bind failed: {0}")]
    TcpBind(io::Error),
    #[error("{0}")]
    Connect(io::Error),
    #[error("Could not get peer address: {0}")]
    PeerAddress(io::Error),
    #[error("Load native certs: {0}")]
    NativeCerts(io::Error),
    #[error("Creating an empty CA stack failed")]
    NewCaStack(io::Error),
    #[error("Could not push intermediate certificate onto stack")]
    CaStackPush(io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio_rustls::TlsConnector;

    const SERVER_NAME: &str = "localhost";
    const PASSWORD: &str = "nopass";

    async fn assert_send_and_recv(cc: TlsConfig, sc: TlsConfig) {
        let send = "foobar";
        let addr = testify::next_addr();
        let mut listener = MaybeTlsListener::bind(&addr, &Some(sc)).await.unwrap();

        let cc = cc.client_config().unwrap();
        tokio::spawn(async move {
            let connector = TlsConnector::from(Arc::new(cc));
            let sock = TcpStream::connect(addr).await.unwrap();
            let mut stream = connector
                .connect(SERVER_NAME.try_into().unwrap(), sock)
                .await
                .unwrap();
            stream.write_all(send.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
            stream.shutdown().await.unwrap();
        });

        let mut incoming = listener.accept().await.unwrap();
        let mut received = String::new();
        incoming.read_to_string(&mut received).await.unwrap();
        assert_eq!(send, received);
    }

    #[tokio::test]
    async fn server() {
        let ca_path = "tests/fixtures/ca/intermediate/certs/ca-chain.cert.pem".into();
        let cert_path: PathBuf = "tests/fixtures/ca/intermediate/certs/localhost.cert.pem".into();
        let key_path = "tests/fixtures/ca/intermediate/private/localhost.key.pem".into();
        let nopass_key_path =
            "tests/fixtures/ca/intermediate/private/localhost.nopass.key.pem".into();

        let cc = TlsConfig {
            ca: Some(ca_path),
            ..Default::default()
        };

        let sc = TlsConfig {
            cert: Some(cert_path.clone()),
            key: Some(key_path),
            key_pass: Some(PASSWORD.into()),
            ..Default::default()
        };

        // with password
        assert_send_and_recv(cc.clone(), sc).await;

        // without password
        let sc = TlsConfig {
            cert: Some(cert_path),
            key: Some(nopass_key_path),
            ..Default::default()
        };
        assert_send_and_recv(cc, sc).await;
    }
}
