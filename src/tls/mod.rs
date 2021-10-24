mod settings;
mod maybe_tls;
mod incoming;
pub mod http;

// re-export
pub use settings::{TLSConfig, TLSSettings, MaybeTLSSettings, MaybeTLSListener};
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
    #[snafu(display("Could not read private key in {:?}, err: {}", filename, source))]
    ReadPemFailed {
        filename: PathBuf,
        source: std::io::Error
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::net::SocketAddr;
    use hyper::{Body, Request, Response, Server, Uri};
    use hyper::service::{make_service_fn, service_fn};
    use testify::next_addr;
    use crate::tls::http::{HttpsConnectorBuilder};
    use super::*;

    #[test]
    fn maybe_tls_settings() {}

    async fn echo_handle(_: Request<Body>) -> Result<Response<Body>, Infallible> {
        Ok(Response::new("Hello, World!\n".into()))
    }

    async fn setup_server(conf: &Option<TLSConfig>) -> SocketAddr {
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

        addr
    }

    #[tokio::test]
    async fn none_tls() {
        let conf = None;
        let addr = setup_server(&conf).await;

        let client = hyper::Client::new();
        let uri = format!("http://{}", addr).parse::<Uri>().unwrap();
        let res = client.get(uri)
            .await
            .unwrap();
        assert_eq!(res.status(), 200);

        let buf = hyper::body::to_bytes(res).await.unwrap();
        assert_eq!(buf, "Hello, World!\n");
    }

    #[tokio::test]
    async fn tls() {
        let conf = TLSConfig::test_options();
        let addr = setup_server(&Some(conf)).await;

        let roots = rustls::RootCertStore::empty();

        let tls = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();

        let https = HttpsConnectorBuilder::new()
            .with_tls_config(tls)
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build();

        let client: hyper::Client<_, hyper::Body> = hyper::Client::builder()
            .build(https);

        let uri = format!("https://{}", addr).parse::<Uri>().unwrap();

        println!("uri {}", uri.host().unwrap());

        let res = client.get(uri)
            .await
            .unwrap();
        assert_eq!(res.status(), 200);

        let buf = hyper::body::to_bytes(res).await.unwrap();
        assert_eq!(buf, "Hello, World!\n");
    }
}

mod tls_tests {


}