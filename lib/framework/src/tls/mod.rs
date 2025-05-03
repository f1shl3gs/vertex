mod incoming;
mod maybe_tls;
mod outgoing;
mod settings;

use std::fmt::Debug;
use std::path::PathBuf;

pub use incoming::{MaybeTlsIncomingStream, MaybeTlsListener};
pub use maybe_tls::MaybeTls;
pub use outgoing::MaybeTlsStream;
pub use settings::TlsConfig;

#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    #[error("Could not read {note} file {filename:?}: {err}")]
    FileReadFailed {
        note: &'static str,
        filename: PathBuf,
        err: std::io::Error,
    },
    #[error("Identity certificate is missing a key")]
    MissingKey,
    #[error("Certificate file contains no certificates")]
    MissingCertificate,
    #[error("Certificate and PrivateKey must be set")]
    MissingCertAndKey,
    #[error("Could not parse certificate in {filename:?}: {err}")]
    CertificateParse {
        filename: PathBuf,
        err: std::io::Error,
    },
    #[error("Could not parse private key in {filename:?}: {err}")]
    PrivateKeyParse {
        filename: PathBuf,
        err: std::io::Error,
    },
    #[error("TLS handshake failed: {0}")]
    Handshake(std::io::Error),
    #[error("Incoming listener failed: {0}")]
    IncomingListener(std::io::Error),
    #[error("Invalid Server Name")]
    InvalidServerName,
    #[error("Error building TLS config: {0}")]
    TlsBuild(rustls::Error),
    #[error("Error adding a certificate to a store: {0}")]
    AddCertToStore(rustls::Error),
    #[error("{0}")]
    VerifierBuild(rustls::client::VerifierBuilderError),
    #[error("TCP bind failed: {0}")]
    TcpBind(std::io::Error),
    #[error(transparent)]
    Connect(std::io::Error),
    #[error("Load native certs: {0}")]
    NativeCerts(std::io::Error),
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
        let mut listener = MaybeTlsListener::bind(&addr, Some(&sc)).await.unwrap();

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
        let ca_path = "tests/ca/intermediate/certs/ca-chain.cert.pem".into();
        let cert_path: PathBuf = "tests/ca/intermediate/certs/localhost.cert.pem".into();
        let key_path = "tests/ca/intermediate/private/localhost.key.pem".into();
        let nopass_key_path = "tests/ca/intermediate/private/localhost.nopass.key.pem".into();

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
