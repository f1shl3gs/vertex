mod trace;

use std::borrow::Cow;
use std::{
    fmt,
    task::{Context, Poll},
};

use configurable::Configurable;
use futures::future::BoxFuture;
use headers::{Authorization, HeaderMapExt};
use http::{header, header::HeaderValue, request::Builder, uri::InvalidUri, HeaderMap, Request};
use hyper::{
    body::{Body, HttpBody},
    client,
    client::{Client, HttpConnector},
};
use hyper_openssl::HttpsConnector;
use hyper_proxy::ProxyConnector;
use metrics::{exponential_buckets, Attributes};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tower::Service;
use tracing_futures::Instrument;
use tracing_internal::SpanExt;

use crate::{
    config::ProxyConfig,
    tls::{tls_connector_builder, MaybeTlsSettings, TlsError},
};

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("Failed to build TLS connector: {0}")]
    BuildTlsConnector(#[from] TlsError),
    #[error("Failed to build HTTPS connector: {0}")]
    MakeHttpsConnector(#[from] openssl::error::ErrorStack),
    #[error("Failed to build Proxy connector: {0}")]
    MakeProxyConnector(#[from] InvalidUri),
    #[error("Failed to make HTTP(S) request: {0}")]
    CallRequest(#[from] hyper::Error),
    #[error("Failed to build HTTP request: {0}")]
    BuildRequest(http::Error),
}

pub type HttpClientFuture = <HttpClient as Service<http::Request<Body>>>::Future;

pub struct HttpClient<B = Body> {
    client: Client<ProxyConnector<HttpsConnector<HttpConnector>>, B>,
    user_agent: HeaderValue,
}

impl<B> HttpClient<B>
where
    B: fmt::Debug + HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<crate::Error>,
{
    pub fn new(
        tls_settings: impl Into<MaybeTlsSettings>,
        proxy_config: &ProxyConfig,
    ) -> Result<HttpClient<B>, HttpError> {
        HttpClient::new_with_custom_client(tls_settings, proxy_config, &mut Client::builder())
    }

    pub fn new_with_custom_client(
        tls_settings: impl Into<MaybeTlsSettings>,
        proxy_config: &ProxyConfig,
        client_builder: &mut client::Builder,
    ) -> Result<HttpClient<B>, HttpError> {
        let mut http = HttpConnector::new();
        http.enforce_http(false);

        let settings = tls_settings.into();
        let tls = tls_connector_builder(&settings)?;
        let mut https = HttpsConnector::with_connector(http, tls)?;

        let settings = settings.tls().cloned();
        https.set_callback(move |c, _uri| {
            if let Some(settings) = &settings {
                settings.apply_connect_configuration(c);
            }

            Ok(())
        });

        let mut proxy = ProxyConnector::new(https).unwrap();
        proxy_config.configure(&mut proxy)?;
        let client = client_builder.build(proxy);

        let version = crate::get_version();
        let user_agent = HeaderValue::from_str(&format!("Vertex/{}", version))
            .expect("Invalid header value for version!");

        Ok(HttpClient { client, user_agent })
    }

    pub fn send(
        &self,
        mut req: Request<B>,
    ) -> BoxFuture<'static, Result<http::Response<Body>, HttpError>> {
        let span = tracing::info_span!("http", uri = ?req.uri());
        let _enter = span.enter();

        default_request_headers(&mut req, &self.user_agent);

        // inject tracing data
        trace::inject(span.context(), &mut req);

        let resp = self.client.request(req);

        let fut = async move {
            // Capture the time right before we issue the request.
            // Request doesn't start the processing until we start polling it.
            let before = std::time::Instant::now();

            // Send request and wait for the result.
            let resp_result = resp.await;

            // Compute the roundtrip time it took to send the request and get
            // the response or error.
            let roundtrip = before.elapsed();

            // Handle the errors and extract the response.
            let resp = resp_result.map_err(|err| {
                debug!(
                    message = "HTTP error",
                    %err,
                );

                metrics::register_counter(
                    "http_client_request_errors_total",
                    "The total number of HTTP request errors for this component.",
                )
                .recorder([("error", Cow::from(err.to_string()))])
                .inc(1);
                metrics::register_histogram(
                    "http_client_request_rtt_seconds",
                    "The round-trip time (RTT) of HTTP requests",
                    exponential_buckets(0.01, 2.0, 10),
                )
                .recorder(&[("status", "none")])
                .record(roundtrip.as_secs_f64());

                err
            })?;

            debug!(
                message = "HTTP response received",
                status = %resp.status(),
                version = ?resp.version(),
                headers = ?remove_sensitive(resp.headers()),
                body = %FormatBody(resp.body()),
            );

            let attrs = Attributes::from([("status", resp.status().to_string().into())]);
            metrics::register_counter(
                "http_client_requests_total",
                "The total number of HTTP requests.",
            )
            .recorder(attrs.clone())
            .inc(1);
            metrics::register_histogram(
                "http_client_request_latency_seconds",
                "The round-trip time (RTT) of HTTP requests.",
                exponential_buckets(0.01, 2.0, 10),
            )
            .recorder(attrs)
            .record(roundtrip.as_secs_f64());

            Ok(resp)
        }
        .instrument(span.clone());

        Box::pin(fut)
    }
}

fn default_request_headers<B>(request: &mut Request<B>, user_agent: &HeaderValue) {
    if !request.headers().contains_key("User-Agent") {
        request
            .headers_mut()
            .insert("User-Agent", user_agent.clone());
    }

    if !request.headers().contains_key("Accept-Encoding") {
        // hardcoding until we support compressed responses:
        // https://github.com/timberio/vector/issues/5440
        request
            .headers_mut()
            .insert("Accept-Encoding", HeaderValue::from_static("identity"));
    }
}

/// Newtype placeholder to provide a formatter for the request and response body.
struct FormatBody<'a, B>(&'a B);

impl<'a, B: HttpBody> std::fmt::Display for FormatBody<'a, B> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let size = self.0.size_hint();
        match (size.lower(), size.upper()) {
            (0, None) => write!(fmt, "[unknown]"),
            (lower, None) => write!(fmt, "[>={} bytes]", lower),

            (0, Some(0)) => write!(fmt, "[empty]"),
            (0, Some(upper)) => write!(fmt, "[<={} bytes]", upper),

            (lower, Some(upper)) if lower == upper => write!(fmt, "[{} bytes]", lower),
            (lower, Some(upper)) => write!(fmt, "[{}..={} bytes]", lower, upper),
        }
    }
}

fn remove_sensitive(headers: &HeaderMap<HeaderValue>) -> HeaderMap<HeaderValue> {
    let mut headers = headers.clone();
    for name in &[
        header::AUTHORIZATION,
        header::PROXY_AUTHORIZATION,
        header::COOKIE,
        header::SET_COOKIE,
    ] {
        if let Some(value) = headers.get_mut(name) {
            value.set_sensitive(true);
        }
    }

    headers
}

impl<B> Service<Request<B>> for HttpClient<B>
where
    B: fmt::Debug + HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<crate::Error> + Send,
{
    type Response = http::Response<Body>;
    type Error = HttpError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        self.send(request)
    }
}

impl<B> Clone for HttpClient<B> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            user_agent: self.user_agent.clone(),
        }
    }
}

impl<B> fmt::Debug for HttpClient<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpClient")
            .field("client", &self.client)
            .field("user_agent", &self.user_agent)
            .finish()
    }
}

/// The authentication strategy for http request/response
#[derive(Configurable, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "strategy")]
pub enum Auth {
    /// Basic authentication.
    ///
    /// The username and password are concatenated and encoded via [base64][base64].
    ///
    /// [base64]: https://en.wikipedia.org/wiki/Base64
    Basic {
        /// The basic authentication username.
        #[configurable(required)]
        user: String,

        /// The basic authentication password.
        #[configurable(required)]
        password: String,
    },

    /// Bearer authentication.
    ///
    /// The bearer token value (OAuth2, JWT, etc) is passed as-is.
    Bearer {
        /// The bearer authentication token.
        #[configurable(required)]
        token: String,
    },
}

impl Auth {
    pub fn basic(user: String, password: String) -> Self {
        Self::Basic { user, password }
    }
}

pub trait MaybeAuth: Sized {
    fn choose_one(&self, other: &Self) -> crate::Result<Self>;
}

impl MaybeAuth for Option<Auth> {
    fn choose_one(&self, other: &Self) -> crate::Result<Self> {
        if self.is_some() && other.is_some() {
            Err("Two authorization credentials was provided.".into())
        } else {
            Ok(self.clone().or_else(|| other.clone()))
        }
    }
}

impl Auth {
    pub fn apply<B>(&self, req: &mut Request<B>) {
        self.apply_headers_map(req.headers_mut())
    }

    pub fn apply_builder(&self, mut builder: Builder) -> Builder {
        if let Some(map) = builder.headers_mut() {
            self.apply_headers_map(map)
        }
        builder
    }

    pub fn apply_headers_map(&self, map: &mut HeaderMap) {
        match &self {
            Auth::Basic { user, password } => {
                let auth = Authorization::basic(user, password);
                map.typed_insert(auth);
            }
            Auth::Bearer { token } => match Authorization::bearer(token) {
                Ok(auth) => map.typed_insert(auth),
                Err(err) => error!(message = "Invalid bearer token.", token = %token, %err),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_request_headers_defaults() {
        let user_agent = HeaderValue::from_static("vertex");
        let mut request = Request::post("http://example.com").body(()).unwrap();
        default_request_headers(&mut request, &user_agent);
        assert_eq!(
            request.headers().get("Accept-Encoding"),
            Some(&HeaderValue::from_static("identity")),
        );
        assert_eq!(request.headers().get("User-Agent"), Some(&user_agent));
    }

    #[test]
    fn test_default_request_headers_does_not_overwrite() {
        let mut request = Request::post("http://example.com")
            .header("Accept-Encoding", "gzip")
            .header("User-Agent", "foo")
            .body(())
            .unwrap();
        default_request_headers(&mut request, &HeaderValue::from_static("vector"));
        assert_eq!(
            request.headers().get("Accept-Encoding"),
            Some(&HeaderValue::from_static("gzip")),
        );
        assert_eq!(
            request.headers().get("User-Agent"),
            Some(&HeaderValue::from_static("foo"))
        );
    }
}
