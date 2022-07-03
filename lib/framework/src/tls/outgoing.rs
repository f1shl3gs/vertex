use std::{net::SocketAddr, pin::Pin};

use tokio::net::TcpStream;
use tokio_openssl::SslStream;

use super::{tls_connector, MaybeTlsSettings, MaybeTlsStream};
use crate::tls::TlsError;

impl MaybeTlsSettings {
    pub async fn connect(
        &self,
        host: &str,
        addr: &SocketAddr,
    ) -> crate::tls::Result<MaybeTlsStream<TcpStream>> {
        let stream = TcpStream::connect(addr).await.map_err(TlsError::Connect)?;

        match self {
            MaybeTlsSettings::Raw(()) => Ok(MaybeTlsStream::Raw(stream)),
            MaybeTlsSettings::Tls(_) => {
                let config = tls_connector(self)?;
                let ssl = config.into_ssl(host).map_err(TlsError::SslBuild)?;

                let mut stream = SslStream::new(ssl, stream).map_err(TlsError::SslBuild)?;
                Pin::new(&mut stream)
                    .connect()
                    .await
                    .map_err(TlsError::Handshake)?;

                debug!(message = "Negotiated TLS.");

                Ok(MaybeTlsStream::Tls(stream))
            }
        }
    }
}
