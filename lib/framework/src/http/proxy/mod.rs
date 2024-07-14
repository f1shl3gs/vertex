mod rt;
mod stream;
mod tunnel;

use std::env;
use std::fmt::Formatter;
use std::future::Future;
use std::io::{Error, ErrorKind};
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use configurable::schema::{generate_array_schema, SchemaGenerator, SchemaObject};
use configurable::{Configurable, GenerateError};
use futures_util::TryFutureExt;
use http::{HeaderMap, Uri};
use hyper::rt::{Read, Write};
use hyper_rustls::ConfigBuilderExt;
use hyper_util::rt::TokioIo;
use ipnet::IpNet;
use rustls::pki_types::ServerName;
use rustls::ClientConfig;
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use stream::ProxyStream;
use tokio_rustls::TlsConnector;
use tower::Service;
use tunnel::TunnelConnect;

/// Represents a possible matching entry for an IP address
#[derive(Clone, Debug, PartialEq)]
enum Ip {
    Address(IpAddr),
    Network(IpNet),
}

/// A wrapper around a list of IP cidr blocks or addresses with a [IpMatcher::contains]
/// method for checking if an IP address is contained within the matcher.
#[derive(Clone, Debug, Default, PartialEq)]
struct IpMatcher(Vec<Ip>);

impl IpMatcher {
    fn contains(&self, addr: IpAddr) -> bool {
        for ip in &self.0 {
            match ip {
                Ip::Address(address) => {
                    if &addr == address {
                        return true;
                    }
                }
                Ip::Network(net) => {
                    if net.contains(&addr) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

/// A wrapper around a list of domains with a [DomainMatcher::contains] method for
/// checking if a domain is contained within the matcher.
#[derive(Clone, Debug, Default, PartialEq)]
struct DomainMatcher(Vec<String>);

impl DomainMatcher {
    // The following links may be useful to understand the origin of these rules:
    // * https://curl.se/libcurl/c/CURLOPT_NOPROXY.html
    // * https://github.com/curl/curl/issues/1208
    fn contains(&self, domain: &str) -> bool {
        let domain_len = domain.len();

        for d in &self.0 {
            if d == domain || d.strip_prefix('.') == Some(domain) {
                return true;
            }

            if domain.ends_with(d) {
                if d.starts_with('.') {
                    // If the first character of d is a dot, that means the first
                    // character of domain must also be a dot, so we are looking at
                    // a subdomain of d and that matches
                    return true;
                }

                if domain.as_bytes().get(domain_len - d.len() - 1) == Some(&b'.') {
                    // Given that d is a prefix of domain, if the prior character
                    // in domain is a dot then means we must be matching a subdomain
                    // of d, and that matches
                    return true;
                }
            } else if d == "*" {
                return true;
            }
        }

        false
    }
}

/// A configuration for filtering out requests that shouldn't be proxied
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NoProxy {
    domains: DomainMatcher,
    ips: IpMatcher,
}

impl From<&str> for NoProxy {
    fn from(value: &str) -> Self {
        Self::from_iterator(value.split(','))
    }
}

impl Configurable for NoProxy {
    fn generate_schema(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        generate_array_schema::<String>(gen)
    }
}

impl Serialize for NoProxy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.ips.0.len() + self.domains.0.len()))?;
        for domain in &self.domains.0 {
            seq.serialize_element(domain)?;
        }

        for ip in &self.ips.0 {
            let s = match ip {
                Ip::Address(addr) => addr.to_string(),
                Ip::Network(net) => net.to_string(),
            };
            seq.serialize_element(&s)?;
        }

        seq.end()
    }
}

impl<'de> Deserialize<'de> for NoProxy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NoProxyVisitor;

        impl<'de> Visitor<'de> for NoProxyVisitor {
            type Value = Vec<String>;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string or list of strings")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v.split(',').map(|item| item.trim().to_string()).collect())
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                Deserialize::deserialize(serde::de::value::SeqAccessDeserializer::new(seq))
            }
        }

        deserializer
            .deserialize_any(NoProxyVisitor)
            .map(|items| NoProxy::from_iterator(items.iter()))
    }
}

impl NoProxy {
    /// Returns a new no-proxy configuration based on environment variables (or `None`
    /// if no variables are set).
    pub fn from_env() -> Option<Self> {
        let raw = env::var("NO_PROXY")
            .or_else(|_err| env::var("no_proxy"))
            .unwrap_or_default();

        Some(Self::from_iterator(raw.split(',')))
    }

    /// Returns a new no-proxy configuration based on a `no_proxy` string (or `None`
    /// if no variables are set)
    ///
    /// The rules are as follows:
    /// * The environment variable `NO_PROXY` is checked, if it is not set, `no_proxy`
    /// is checked.
    /// * If neither environment variable is set, `None` is returned.
    /// * Entries are expected to be comma-separated (whitespace between entries is ignored)
    /// * IP addresses (both IPv4 and IPv6) are allowed, as are optional subnet masks (by
    /// adding /size, for example "`192.168.1.0/24`").
    /// * An entry "`*`" matches all hostnames (this is the only wildcard allowed)
    /// * Any other entry is considered a domain name (and may contain a leading dot, for
    /// example `google.com` and `.google.com` are equivalent) and would match both that
    /// domain and all subdomains.
    ///
    /// For example, if `"NO_PROXY=google.com, 192.168.1.0/24"` was set, all of the
    /// following would match (and therefore would bypass the proxy):
    /// * `http://google.com/`
    /// * `http://www.google.com/`
    /// * `http://192.168.1.42/`
    ///
    /// The URL `http://notgoogle.com/` would not match.
    fn from_iterator<V: AsRef<str>, I: Iterator<Item = V>>(parts: I) -> Self {
        let mut ips = vec![];
        let mut domains = vec![];
        for part in parts {
            let part = part.as_ref().trim();
            match part.parse::<IpNet>() {
                // If we can parse an IP net or address, then use it, otherwise,
                // assume it is a domain.
                Ok(ip) => ips.push(Ip::Network(ip)),
                Err(_err) => match part.parse::<IpAddr>() {
                    Ok(addr) => ips.push(Ip::Address(addr)),
                    Err(_err) => domains.push(part.to_string()),
                },
            }
        }

        NoProxy {
            ips: IpMatcher(ips),
            domains: DomainMatcher(domains),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ips.0.is_empty() && self.domains.0.is_empty()
    }

    pub fn matches(&self, host: &str) -> bool {
        // According to RFC3986, raw IPv6 hosts will be wrapped in []. So we need to
        // strip those off the end in order to parse correctly.
        let host = if host.starts_with('[') {
            let x: &[_] = &['[', ']'];
            host.trim_matches(x)
        } else {
            host
        };

        match host.parse::<IpAddr>() {
            // If we can parse an IP addr, then use it, otherwise, assume it is a domain
            Ok(ip) => self.ips.contains(ip),
            Err(_err) => self.domains.contains(host),
        }
    }
}

/// A Custom struct to proxy custom uris
#[derive(Clone)]
pub struct Custom(Arc<dyn Fn(Option<&str>, Option<&str>, Option<u16>) -> bool + Send + Sync>);

impl<F: Fn(Option<&str>, Option<&str>, Option<u16>) -> bool + Send + Sync + 'static> From<F>
    for Custom
{
    fn from(f: F) -> Custom {
        Custom(Arc::new(f))
    }
}

/// The intercept enum to filter connections.
#[derive(Clone)]
pub enum Intercept {
    /// All incoming connection will go through proxy
    All,
    /// Only http connections will go through proxy
    Http,
    /// Only https connections will go through proxy
    Https,
    /// No connection will go through proxy
    None,
    /// A custom intercept
    Custom(Custom),
}

impl Intercept {
    /// A function to check if given Uri is proxied
    #[inline]
    fn matches(&self, uri: &Uri) -> bool {
        match (self, uri.scheme_str()) {
            (&Intercept::All, _)
            | (&Intercept::Http, Some("http"))
            | (&Intercept::Https, Some("https")) => true,
            (&Intercept::Custom(Custom(ref f)), _) => {
                f(uri.scheme_str(), uri.host(), uri.port_u16())
            }
            _ => false,
        }
    }
}

/// Configuration of a proxy that a `Client` should pass requests to.
///
/// A `Proxy` has a couple pieces to it:
///
/// - a URL of how to talk to the proxy
/// - rules on what `Client` requests should be directed to the proxy
///
/// For instance, let's look at `Proxy::http`:
#[derive(Clone)]
pub struct Proxy {
    intercept: Intercept,
    headers: HeaderMap,
    uri: Uri,
}

impl Proxy {
    /// Create a new `Proxy`
    pub fn new<I: Into<Intercept>>(intercept: I, uri: Uri) -> Self {
        Self {
            intercept: intercept.into(),
            headers: Default::default(),
            uri,
        }
    }
}

/// A wrapper around `Proxy`s with a connector.
#[derive(Clone)]
pub struct ProxyConnector<C> {
    connector: C,
    proxies: Vec<Proxy>,

    tls: Option<TlsConnector>,
}

impl<C> ProxyConnector<C> {
    /// Create a new secured Proxies
    pub fn new(connector: C) -> Result<Self, Error> {
        let config = ClientConfig::builder()
            .with_native_roots()?
            .with_no_client_auth();

        Ok(ProxyConnector {
            connector,
            proxies: vec![],
            tls: Some(TlsConnector::from(Arc::new(config))),
        })
    }

    fn match_proxy(&self, uri: &Uri) -> Option<&Proxy> {
        self.proxies.iter().find(|p| p.intercept.matches(uri))
    }

    /// Add a new additional proxy
    pub fn add_proxy(&mut self, proxy: Proxy) {
        self.proxies.push(proxy)
    }
}

impl<C> Service<Uri> for ProxyConnector<C>
where
    C: Service<Uri>,
    C::Response: Read + Write + Send + Unpin + 'static,
    C::Future: Send + 'static,
    C::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = ProxyStream<C::Response>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.connector.poll_ready(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(err)) => Poll::Ready(Err(Error::new(ErrorKind::Other, err))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        if let (Some(proxy), Some(host)) = (self.match_proxy(&uri), uri.host()) {
            if uri.scheme() == Some(&http::uri::Scheme::HTTPS) {
                let host = host.to_owned();
                let port =
                    uri.port_u16()
                        .unwrap_or(if uri.scheme() == Some(&http::uri::Scheme::HTTP) {
                            80
                        } else {
                            443
                        });
                let tunnel = TunnelConnect::new(&host, port, &proxy.headers);
                let connection =
                    proxy_dst(&uri, &proxy.uri).map(|proxy_url| self.connector.call(proxy_url));
                let tls = if uri.scheme() == Some(&http::uri::Scheme::HTTPS) {
                    self.tls.clone()
                } else {
                    None
                };

                Box::pin(async move {
                    #[allow(clippy::never_loop)]
                    loop {
                        let proxy_stream = match match connection {
                            Ok(v) => v,
                            Err(err) => break Err(err),
                        }
                        .await
                        .map_err(|err| Error::new(ErrorKind::Other, err))
                        {
                            Ok(v) => v,
                            Err(err) => break Err(err),
                        };
                        let tunnel_stream = tunnel.with_stream(proxy_stream).await?;

                        break match tls {
                            Some(tls) => {
                                let dns_ref = match ServerName::try_from(host)
                                    .map_err(|err| Error::new(ErrorKind::Other, err))
                                {
                                    Ok(v) => v,
                                    Err(err) => break Err(err),
                                };
                                let secure_stream = match tls
                                    .connect(dns_ref, TokioIo::new(tunnel_stream))
                                    .await
                                    .map_err(|err| Error::new(ErrorKind::Other, err))
                                {
                                    Ok(v) => v,
                                    Err(err) => break Err(err),
                                };

                                Ok(ProxyStream::Secured(TokioIo::new(secure_stream)))
                            }

                            None => Ok(ProxyStream::Regular(tunnel_stream)),
                        };
                    }
                })
            } else {
                match proxy_dst(&uri, &proxy.uri) {
                    Ok(proxy_uri) => Box::pin(
                        self.connector
                            .call(proxy_uri)
                            .map_ok(ProxyStream::Regular)
                            .map_err(|err| Error::new(ErrorKind::Other, err)),
                    ),
                    Err(err) => Box::pin(futures::future::err(Error::new(ErrorKind::Other, err))),
                }
            }
        } else {
            Box::pin(
                self.connector
                    .call(uri)
                    .map_ok(ProxyStream::NoProxy)
                    .map_err(|err| Error::new(ErrorKind::Other, err)),
            )
        }
    }
}

fn proxy_dst(dst: &Uri, proxy: &Uri) -> Result<Uri, Error> {
    Uri::builder()
        .scheme(proxy.scheme_str().ok_or_else(|| {
            Error::new(
                ErrorKind::Other,
                format!("proxy uri missing scheme: {}", proxy),
            )
        })?)
        .authority(
            proxy
                .authority()
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Other,
                        format!("proxy uri missing host: {}", proxy),
                    )
                })?
                .clone(),
        )
        .path_and_query(dst.path_and_query().unwrap().clone())
        .build()
        .map_err(|err| Error::new(ErrorKind::Other, format!("other error: {}", err)))
}
