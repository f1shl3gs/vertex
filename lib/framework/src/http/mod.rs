mod events;

// re-export
pub use events::*;

use std::{
    fmt,
    task::{Context, Poll},
};

use futures::future::BoxFuture;
use headers::{Authorization, HeaderMapExt};
use http::{header::HeaderValue, request::Builder, uri::InvalidUri, HeaderMap, Request};
use hyper::{
    body::{Body, HttpBody},
    client,
    client::{Client, HttpConnector},
};
use hyper_openssl::HttpsConnector;
use hyper_proxy::ProxyConnector;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use tower::Service;
use tracing_futures::Instrument;

use crate::config::GenerateConfig;
use crate::{
    config::ProxyConfig,
    tls::{tls_connector_builder, MaybeTlsSettings, TlsError},
};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum HttpError {
    #[snafu(display("Failed to build TLS connector: {}", source))]
    BuildTlsConnector { source: TlsError },
    #[snafu(display("Failed to build HTTPS connector: {}", source))]
    MakeHttpsConnector { source: openssl::error::ErrorStack },
    #[snafu(display("Failed to build Proxy connector: {}", source))]
    MakeProxyConnector { source: InvalidUri },
    #[snafu(display("Failed to make HTTP(S) request: {}", source))]
    CallRequest { source: hyper::Error },
    #[snafu(display("Failed to build HTTP request: {}", source))]
    BuildRequest { source: http::Error },
}

pub type HttpClientFuture = <HttpClient as Service<http::Request<Body>>>::Future;

pub struct HttpClient<B = Body> {
    client: Client<ProxyConnector<HttpsConnector<HttpConnector>>, B>,
    user_agent: HeaderValue,
    // metrics
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
        let tls = tls_connector_builder(&settings).context(BuildTlsConnectorSnafu)?;
        let mut https =
            HttpsConnector::with_connector(http, tls).context(MakeHttpsConnectorSnafu)?;

        let settings = settings.tls().cloned();
        https.set_callback(move |c, _uri| {
            if let Some(settings) = &settings {
                settings.apply_connect_configuration(c);
            }

            Ok(())
        });

        let mut proxy = ProxyConnector::new(https).unwrap();
        proxy_config
            .configure(&mut proxy)
            .context(MakeProxyConnectorSnafu)?;
        let client = client_builder.build(proxy);

        let version = crate::get_version();
        let user_agent = HeaderValue::from_str(&format!("Vertex/{}", version))
            .expect("Invalid header value for version!");

        Ok(HttpClient { client, user_agent })
    }

    pub fn send(
        &self,
        mut request: Request<B>,
    ) -> BoxFuture<'static, Result<http::Response<Body>, HttpError>> {
        let span = tracing::info_span!("http");
        let _enter = span.enter();

        default_request_headers(&mut request, &self.user_agent);

        emit!(&AboutToSendHttpRequest { request: &request });

        let response = self.client.request(request);

        let fut = async move {
            // Capture the time right before we issue the request.
            // Request doesn't start the processing until we start polling it.
            let before = std::time::Instant::now();

            // Send request and wait for the result.
            let resp_result = response.await;

            // Compute the roundtrip time it took to send the request and get
            // the response or error.
            let roundtrip = before.elapsed();

            // Handle the errors and extract the response.
            let resp = resp_result
                .map_err(|error| {
                    // Emit the error into the internal events system.
                    emit!(&GotHttpError {
                        error: &error,
                        roundtrip
                    });
                    error
                })
                .context(CallRequestSnafu)?;

            debug!(
                message = "HTTP response received",
                status = %resp.status(),
                version = ?resp.version(),
                headers = ?remove_sensitive(resp.headers()),
                body = %FormatBody(resp.body()),
            );

            metrics::register_counter(
                "http_client_requests_total",
                "The total number of HTTP requests.",
            )
            .recorder(&[("status", resp.status().as_str())])
            .inc(1);
            metrics::register_histogram(
                "http_client_request_latency_seconds",
                "The round-trip time (RTT) of HTTP requests.",
            )
            .recorder(&[("status", resp.status().as_str())])
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

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "strategy")]
pub enum Auth {
    Basic { user: String, password: String },
    Bearer { token: String },
}

impl GenerateConfig for Auth {
    fn generate_config() -> String {
        r#"
# The authentication strategy to use, the available value
# is "basic" or "bearer". If strategy is set to "bearer",
# "user" and "password" is ignored, and "token" must be
# configured.
strategy: basic

# The basic authentication user name.
user: username

# The basic authentication password.
password: password

# The token to use for bearer authentication.
# token: abcdefg
"#
        .into()
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
