use std::fmt;
use hyper::{Body, Client, http};
use hyper::body::HttpBody;
use hyper::client::HttpConnector;
use hyper::header::HeaderValue;
use hyper::http::uri::InvalidUri;
use hyper::service::Service;
use hyper_proxy::ProxyConnector;
use hyper_rustls::HttpsConnector;
use crate::tls::{MaybeTLS, MaybeTLSSettings, TLSError};

#[derive(Debug, Snafu)]
pub enum HTTPError {
    #[snafu(display("Failed to build TLS connector: {}", source))]
    BuildTLSConnector { source: TLSError },
    #[snafu(display("Failed to build HTTPS connector: {}", source))]
    BuildHTTPSConnector { source: rustls::TLSError },
    #[snafu(display("Failed to build Proxy connector: {}", source))]
    BuildProxyConnector { source: InvalidUri },
    #[snafu(display("Failed to make HTTP(S) request: {}", source))]
    CallRequest { source: hyper::Error },
}

pub struct HTTPClient<B = Body> {
    client: Client<ProxyConnector<HttpsConnector<HttpConnector>>, B>,
    user_agent: HeaderValue,
}

impl<B> HTTPClient<B>
    where
        B: fmt::Debug + HttpBody + Send + 'static,
        B::Data: Send,
        B::Error: Into<crate::Error>
{
    pub fn new(
        tls_setting: impl Into<MaybeTLSSettings>,
        proxy_config: &ProxyConfig,
    ) -> Result<HTTPClient<B>, HTTPError> {

    }
}

pub type HTTPClientFuture = <HTTPClient as Service<http::Request<Body>>>::Future;

