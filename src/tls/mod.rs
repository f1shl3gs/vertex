mod settings;
mod maybe_tls;
mod incoming;

pub use settings::{TLSConfig, MaybeTLSSettings, MaybeTLSListener};
pub use maybe_tls::{
    MaybeTLS,
};

use std::path::PathBuf;
use snafu::Snafu;

pub type MaybeTLSStream<S> = MaybeTLS<S, tokio_rustls::TlsStream<S>>;

#[derive(Debug, Snafu)]
pub enum TLSError {
    #[snafu(display("Could not open {} file {:?}: {}", note, filename, source))]
    FileOpenFailed {
        note: &'static str,
        filename: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("Incoming listener failed: {}", source))]
    IncomingListener { source: tokio::io::Error },

    #[snafu(display("Handshake failed: {}", source))]
    Handshake { source: std::io::Error },
    #[snafu(display("TCP bind failed: {}", source))]
    TcpBind { source: tokio::io::Error },

    #[snafu(display("TLS configuration requires a certificate when enabled"))]
    MissingRequiredIdentity,
    #[snafu(display("Creating the TLS acceptor failed: {}", source))]
    CreateAcceptor { source: std::io::Error },

    // TLS
    #[snafu(display("Could not parse certificate in {:?}", filename))]
    CertificateParseError { filename: PathBuf },
    #[snafu(display("Could not parse private key in {:?}", filename))]
    PrivateKeyParseError {
        filename: PathBuf
    },
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use hyper::{Body, Request, Response, Server};
    use hyper::server::conn::AddrStream;
    use hyper::service::{make_service_fn, service_fn};
    use testify::next_addr;
    use super::*;

    #[test]
    fn maybe_tls_settings() {}

    async fn echo_handle(_: Request<Body>) -> Result<Response<Body>, Infallible> {
        Ok(Response::new("Hello, World!\n".into()))
    }

    #[tokio::test]
    async fn none_tls() {
        let conf = None;
        let tls = MaybeTLSSettings::from_config(&conf)
            .unwrap();

        let addr = next_addr();
        let listener = tls.bind(&addr)
            .await
            .unwrap();

        let service = make_service_fn(|_conn| async {
            Ok::<_, Infallible>(service_fn(echo_handle))
        });

        tokio::spawn(async move {
            Server::builder(hyper::server::accept::from_stream(listener.accept_stream()))
                .serve(service)
                .await
                .unwrap();
        });
    }
}