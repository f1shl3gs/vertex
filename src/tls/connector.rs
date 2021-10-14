use futures::FutureExt;
use hyper::client::connect::HttpConnector;
use hyper::{client::connect::Connection, service::Service, Uri};
use rustls::ClientConfig;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{fmt, io};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::TlsConnector;
use webpki::DNSNameRef;
use super::stream::MaybeHTTPSStream;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// A Connector for the `https` scheme.
#[derive(Clone)]
pub struct HTTPSConnector<T> {
    http: T,
    tls_config: Arc<ClientConfig>,
}

impl HTTPSConnector<HttpConnector> {
    /// Construct a new `HttpsConnector` using the OS root store
    pub fn with_native_roots() -> Self {
        let mut config = ClientConfig::new();
        config.root_store = match rustls_native_certs::load_native_certs() {
            Ok(store) => store,
            Err((Some(store), err)) => {
                warn!("Could not load all certificates: {:?}", err);
                store
            }
            Err((None, err)) => Err(err).expect("cannot access native cert store"),
        };
        if config.root_store.is_empty() {
            panic!("no CA certificates found");
        }
        Self::build(config)
    }

    /// Construct a new `HttpsConnector` using the `webpki_roots`
    pub fn with_webpki_roots() -> Self {
        let mut config = ClientConfig::new();
        config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        Self::build(config)
    }

    fn build(mut config: ClientConfig) -> Self {
        let mut http = HttpConnector::new();
        http.enforce_http(false);

        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        config.ct_logs = Some(&ct_logs::LOGS);
        (http, config).into()
    }
}

impl<T> fmt::Debug for HTTPSConnector<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("HttpsConnector").finish()
    }
}

impl<H, C> From<(H, C)> for HTTPSConnector<H>
    where
        C: Into<Arc<ClientConfig>>,
{
    fn from((http, cfg): (H, C)) -> Self {
        HTTPSConnector {
            http,
            tls_config: cfg.into(),
        }
    }
}

impl<T> Service<Uri> for HTTPSConnector<T>
    where
        T: Service<Uri>,
        T::Response: Connection + AsyncRead + AsyncWrite + Send + Unpin + 'static,
        T::Future: Send + 'static,
        T::Error: Into<BoxError>,
{
    type Response = MaybeHTTPSStream<T::Response>;
    type Error = BoxError;

    #[allow(clippy::type_complexity)]
    type Future =
    Pin<Box<dyn Future<Output = Result<MaybeHTTPSStream<T::Response>, BoxError>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.http.poll_ready(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
            Poll::Pending => Poll::Pending,
        }
    }

    fn call(&mut self, dst: Uri) -> Self::Future {
        let is_https = dst.scheme_str() == Some("https");

        if !is_https {
            let connecting_future = self.http.call(dst);

            let f = async move {
                let tcp = connecting_future.await.map_err(Into::into)?;

                Ok(MaybeHTTPSStream::HTTP(tcp))
            };
            f.boxed()
        } else {
            let cfg = self.tls_config.clone();
            let hostname = dst.host().unwrap_or_default().to_string();
            let connecting_future = self.http.call(dst);

            let f = async move {
                let tcp = connecting_future.await.map_err(Into::into)?;
                let connector = TlsConnector::from(cfg);
                let dnsname = DNSNameRef::try_from_ascii_str(&hostname)
                    .map_err(|_| io::Error::new(io::ErrorKind::Other, "invalid dnsname"))?;
                let tls = connector
                    .connect(dnsname, tcp)
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                Ok(MaybeHTTPSStream::HTTPS(tls))
            };
            f.boxed()
        }
    }
}