use std::fmt;
use futures::future::BoxFuture;
use headers::HeaderMapExt;
use http::Request;
use snafu::Snafu;
use hyper::{Body, Client, HeaderMap, http};
use hyper::body::HttpBody;
use hyper::client::HttpConnector;
use hyper::header::HeaderValue;
use hyper::http::uri::InvalidUri;
use hyper::service::Service;
use hyper_proxy::ProxyConnector;
use hyper_rustls::HttpsConnector;
use crate::config::ProxyConfig;
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
        todo!()
    }

    pub fn send(
        &self,
        mut req: Request<B>,
    ) -> BoxFuture<'static, Result<http::Response<Body>, HTTPError>> {
        default_request_headers(&mut req, &self.user_agent);

        let resp = self.client.request(req);
    }
}

fn default_request_headers<B>(req: &mut http::Request<B>, ua: &HeaderValue) {
    if !req.headers().contains_key("User-Agent") {
        req
            .headers_mut()
            .insert("User-Agent", ua.clone());
    }

    // TODO: support compressed response
    if !req.headers().contains_key("Accept-Encoding") {
        req
            .headers_mut()
            .insert("Accept-Encoding", HeaderValue::from_static("identity"))
    }
}

pub type HTTPClientFuture = <HTTPClient as Service<http::Request<Body>>>::Future;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "strategy")]
pub enum Auth {
    Basic {
        user: String,
        password: String,
    },

    Bearer {
        token: String
    },
}

impl Auth {
    pub fn apply<B>(&self, req: &mut Request<B>) {
        self.apply_headers_map(req.headers_mut());
    }

    pub fn apply_headers_map(&self, map: &mut HeaderMap) {
        match &self {
            Auth::Basic { user, password } => {
                let auth = headers::Authorization::basic(user, password);
                map.typed_insert(auth);
            }

            Auth::Bearer { token } => match headers::Authorization::bearer(token) {
                Ok(auth) => map.typed_insert(auth),
                Err(err) => error!(
                    message = "Invalid bearer token",
                    token = %token,
                    %err,
                )
            }
        }
    }
}