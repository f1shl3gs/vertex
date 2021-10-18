use std::fmt;
use std::fmt::Formatter;
use serde::{Deserialize, Serialize};
use futures::future::BoxFuture;
use headers::HeaderMapExt;
use http::Request;
use snafu::{ResultExt, Snafu};
use hyper::{Body, Client, HeaderMap, http};
use hyper::body::HttpBody;
use hyper::client::HttpConnector;
use hyper::header::HeaderValue;
use hyper::http::uri::InvalidUri;
use hyper::service::Service;
use hyper_proxy::ProxyConnector;
use hyper_rustls::HttpsConnector;
use crate::config::ProxyConfig;
use crate::tls::{HTTPSConnector, MaybeTLS, MaybeTLSSettings, TLSError};

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
    client: Client<ProxyConnector<HTTPSConnector<HttpConnector>>, B>,
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
        let mut http = HttpConnector::new();
        http.enforce_http(false);

        // TODO: enable HTTPS

        let https = HTTPSConnector::with_native_roots();
        let mut proxy = ProxyConnector::new(https)
            .unwrap();
        proxy_config.configure(&mut proxy)
            .context(BuildProxyConnector)?;
        let client = Client::builder()
            .build(proxy);
        let user_agent = HeaderValue::from_str(&format!("Vector/{}", crate::built_info::PKG_VERSION))
            .expect("invalid header value for version!");

        Ok(HTTPClient {
            client,
            user_agent,
        })
    }

    pub fn send(
        &self,
        mut req: Request<B>,
    ) -> BoxFuture<'static, Result<http::Response<Body>, HTTPError>> {
        default_request_headers(&mut req, &self.user_agent);

        let resp = self.client.request(req);
        let fut = async move {
            // Capture the time right before we issue the request.
            // Request doesn't start the processing until we start polling it.
            let before = std::time::Instant::now();

            // Send request and wait for the result
            let resp_result = resp.await;

            // Compute the roundtrip time it took to send the request and get
            // the response or error
            let roundtrip = before.elapsed();

            // Handle the errors and extract the response
            let resp = resp_result
                .map_err(|err| {
                    // TODO: emit http error
                    err
                })
                .context(CallRequest)?;

            // TODO:
            // Emit the response into the internal events system

            Ok(resp)
        };

        Box::pin(fut)
    }
}

impl<B> Clone for HTTPClient<B> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            user_agent: self.user_agent.clone(),
        }
    }
}

impl<B> fmt::Debug for HTTPClient<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("HTTPClient")
            .field("client", &self.client)
            .field("user_agent", &self.user_agent)
            .finish()
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
            .insert("Accept-Encoding", HeaderValue::from_static("identity"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_request_headers_defaults() {
        let ua = HeaderValue::from_static("vertex");
        let mut req = Request::post("http://example.com")
            .body(())
            .unwrap();
        default_request_headers(&mut req, &ua);

        assert_eq!(req.headers().get("User-Agent"), Some(&ua));
        assert_eq!(
            req.headers().get("Accept-Encoding"),
            Some(&HeaderValue::from_static("identity"))
        );
    }

    #[test]
    fn test_default_request_headers_does_not_overwrite() {
        let mut req = Request::get("http://example.com")
            .header("Accept-Encoding", "gzip")
            .header("User-Agent", "foo")
            .body(())
            .unwrap();
        default_request_headers(&mut req, &HeaderValue::from_static("vertex"));
        assert_eq!(
            req.headers().get("Accept-Encoding"),
            Some(&HeaderValue::from_static("gzip"))
        );
        assert_eq!(
            req.headers().get("User-Agent"),
            Some(&HeaderValue::from_static("foo"))
        )
    }
}