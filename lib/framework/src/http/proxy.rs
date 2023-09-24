use std::env;
use std::fmt::{Formatter, Write};
use std::future::Future;
use std::io::{self, Error, ErrorKind, IoSlice};
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::{Buf, BytesMut};
use configurable::schema::{generate_array_schema, SchemaGenerator, SchemaObject};
use configurable::{Configurable, GenerateError};
use futures::pin_mut;
use futures_util::TryFutureExt;
use http::HeaderMap;
use hyper::client::connect::{Connected, Connection};
use hyper::{service::Service, Uri};
use ipnet::IpNet;
use rustls::{ClientConfig, RootCertStore};
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio_rustls::{client::TlsStream, rustls::ServerName, TlsConnector};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

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

/// A Proxy Stream wrapper
#[allow(clippy::large_enum_variant)]
pub enum ProxyStream<R> {
    NoProxy(R),
    Regular(R),
    Secured(TlsStream<R>),
}

impl<R: AsyncRead + AsyncWrite + Unpin> AsyncRead for ProxyStream<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            ProxyStream::NoProxy(r) => Pin::new(r).poll_read(cx, buf),
            ProxyStream::Regular(r) => Pin::new(r).poll_read(cx, buf),
            ProxyStream::Secured(r) => Pin::new(r).poll_read(cx, buf),
        }
    }
}

impl<R: AsyncRead + AsyncWrite + Unpin> AsyncWrite for ProxyStream<R> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        match self.get_mut() {
            ProxyStream::NoProxy(r) => Pin::new(r).poll_write(cx, buf),
            ProxyStream::Regular(r) => Pin::new(r).poll_write(cx, buf),
            ProxyStream::Secured(r) => Pin::new(r).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.get_mut() {
            ProxyStream::NoProxy(r) => Pin::new(r).poll_flush(cx),
            ProxyStream::Regular(r) => Pin::new(r).poll_flush(cx),
            ProxyStream::Secured(r) => Pin::new(r).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.get_mut() {
            ProxyStream::NoProxy(r) => Pin::new(r).poll_shutdown(cx),
            ProxyStream::Regular(r) => Pin::new(r).poll_shutdown(cx),
            ProxyStream::Secured(r) => Pin::new(r).poll_shutdown(cx),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize, Error>> {
        match self.get_mut() {
            ProxyStream::NoProxy(r) => Pin::new(r).poll_write_vectored(cx, bufs),
            ProxyStream::Regular(r) => Pin::new(r).poll_write_vectored(cx, bufs),
            ProxyStream::Secured(r) => Pin::new(r).poll_write_vectored(cx, bufs),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match self {
            ProxyStream::NoProxy(r) => r.is_write_vectored(),
            ProxyStream::Regular(r) => r.is_write_vectored(),
            ProxyStream::Secured(r) => r.is_write_vectored(),
        }
    }
}

impl<R: AsyncRead + AsyncWrite + Connection + Unpin> Connection for ProxyStream<R> {
    fn connected(&self) -> Connected {
        let mut is_h2 = false;
        let connected = match self {
            ProxyStream::NoProxy(r) => r.connected(),
            ProxyStream::Regular(r) => r.connected().proxy(true),
            ProxyStream::Secured(r) => {
                let (underlying, tls) = r.get_ref();
                is_h2 = tls.alpn_protocol() == Some(b"h2");
                underlying.connected().proxy(true)
            }
        };

        if is_h2 {
            connected.negotiated_h2()
        } else {
            connected
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
    pub fn new(connector: C) -> Result<Self, io::Error> {
        let certs = rustls_native_certs::load_native_certs()
            .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?;
        let mut store = RootCertStore::empty();
        for cert in certs {
            store
                .add(&rustls::Certificate(cert.0))
                .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?;
        }

        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(store)
            .with_no_client_auth();
        let tls = TlsConnector::from(Arc::new(config));

        Ok(ProxyConnector {
            connector,
            proxies: vec![],
            tls: Some(tls),
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
    C::Response: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    C::Future: Send + 'static,
    C::Error: Into<BoxError>,
{
    type Response = ProxyStream<C::Response>;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.connector.poll_ready(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(err)) => Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, err))),
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
                        .map_err(|err| io::Error::new(ErrorKind::Other, err))
                        {
                            Ok(v) => v,
                            Err(err) => break Err(err),
                        };
                        let tunnel_stream = tunnel.with_stream(proxy_stream).await?;

                        break match tls {
                            Some(tls) => {
                                let dnsref = match ServerName::try_from(&*host)
                                    .map_err(|err| Error::new(ErrorKind::Other, err))
                                {
                                    Ok(v) => v,
                                    Err(err) => break Err(err),
                                };
                                let secure_stream = match tls
                                    .connect(dnsref, tunnel_stream)
                                    .await
                                    .map_err(|err| io::Error::new(ErrorKind::Other, err))
                                {
                                    Ok(v) => v,
                                    Err(err) => break Err(err),
                                };

                                Ok(ProxyStream::Secured(secure_stream))
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
                            .map_err(|err| io::Error::new(ErrorKind::Other, err)),
                    ),
                    Err(err) => {
                        Box::pin(futures::future::err(io::Error::new(ErrorKind::Other, err)))
                    }
                }
            }
        } else {
            Box::pin(
                self.connector
                    .call(uri)
                    .map_ok(ProxyStream::NoProxy)
                    .map_err(|err| io::Error::new(ErrorKind::Other, err)),
            )
        }
    }
}

fn proxy_dst(dst: &Uri, proxy: &Uri) -> io::Result<Uri> {
    Uri::builder()
        .scheme(proxy.scheme_str().ok_or_else(|| {
            io::Error::new(
                ErrorKind::Other,
                format!("proxy uri missing scheme: {}", proxy),
            )
        })?)
        .authority(
            proxy
                .authority()
                .ok_or_else(|| {
                    io::Error::new(
                        ErrorKind::Other,
                        format!("proxy uri missing host: {}", proxy),
                    )
                })?
                .clone(),
        )
        .path_and_query(dst.path_and_query().unwrap().clone())
        .build()
        .map_err(|err| io::Error::new(ErrorKind::Other, format!("other error: {}", err)))
}

struct TunnelConnect {
    buf: BytesMut,
}

impl TunnelConnect {
    /// Creates a new tunnel through proxy
    fn new(host: &str, port: u16, headers: &HeaderMap) -> Self {
        let mut buf = BytesMut::new();
        write!(
            buf,
            "CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n"
        )
        .expect("should success");
        for (key, value) in headers {
            let value = value.to_str().unwrap_or_default();
            write!(buf, "{}: {}\r\n", key.as_str(), value).expect("should success");
        }

        write!(buf, "\r\n").expect("should success");

        Self { buf }
    }

    fn with_stream<S>(self, stream: S) -> Tunnel<S> {
        Tunnel {
            buf: self.buf,
            stream: Some(stream),
            state: TunnelState::Writing,
        }
    }
}

enum TunnelState {
    Writing,
    Reading,
}

struct Tunnel<S> {
    buf: BytesMut,
    stream: Option<S>,
    state: TunnelState,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Future for Tunnel<S> {
    type Output = Result<S, io::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.stream.is_none() {
            panic!("must not poll after future is complete");
        }

        let this = self.get_mut();
        loop {
            if let TunnelState::Writing = &this.state {
                let fut = this.stream.as_mut().unwrap().write_buf(&mut this.buf);
                pin_mut!(fut);
                let n = match fut.poll(cx) {
                    Poll::Ready(Ok(n)) => n,
                    Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                    Poll::Pending => return Poll::Pending,
                };

                if !this.buf.has_remaining() {
                    this.state = TunnelState::Reading;
                    this.buf.truncate(0);
                } else if n == 0 {
                    return Poll::Ready(Err(io::Error::new(
                        ErrorKind::Other,
                        "unexpected EOF while tunnel writing",
                    )));
                }
            } else {
                let fut = this.stream.as_mut().unwrap().read_buf(&mut this.buf);
                pin_mut!(fut);
                match fut.poll(cx) {
                    Poll::Ready(Ok(n)) => {
                        if n == 0 {
                            return Poll::Ready(Err(io::Error::new(
                                ErrorKind::Other,
                                "unexpected EOF while tunnel reading",
                            )));
                        }
                    }
                    Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                    Poll::Pending => return Poll::Pending,
                };

                let read = &this.buf[..];
                if read.len() > 12 {
                    if read.starts_with(b"HTTP/1.1 200") || read.starts_with(b"HTTP/1.0 200") {
                        if read.ends_with(b"\r\n\r\n") {
                            return Poll::Ready(Ok(this.stream.take().unwrap()));
                        }
                        // else read more
                    } else {
                        let len = read.len().min(16);
                        return Poll::Ready(Err(io::Error::new(
                            ErrorKind::Other,
                            format!(
                                "unsuccessful tunnel ({})",
                                String::from_utf8_lossy(&read[0..len])
                            ),
                        )));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use tokio::net::{TcpListener, TcpStream};

    fn tunnel<S>(conn: S, host: String, port: u16) -> Tunnel<S> {
        TunnelConnect::new(&host, port, &HeaderMap::new()).with_stream(conn)
    }

    async fn mock_tunnel(status_line: Option<&str>) -> SocketAddr {
        let status_line = status_line.unwrap_or("HTTP/1.1 200 OK\r\n\r\n").to_string();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let connect_expected = format!(
            "CONNECT {0}:{1} HTTP/1.1\r\nHost: {0}:{1}\r\n\r\n",
            addr.ip(),
            addr.port()
        )
        .into_bytes();

        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let n = sock.read(&mut buf).await.unwrap();
            assert_eq!(&connect_expected[..], &buf[..n]);

            sock.write_all(status_line.as_bytes()).await.unwrap();
        });

        addr
    }

    #[tokio::test]
    async fn tunnel_ok() {
        let addr = mock_tunnel(None).await;
        let host = addr.ip().to_string();
        let port = addr.port();

        let _c = TcpStream::connect(addr)
            .await
            .map(|s| tunnel(s, host, port))
            .unwrap();
    }

    #[tokio::test]
    async fn tunnel_eof() {
        let addr = mock_tunnel(Some("HTTP/1.1 200 OK")).await;
        let host = addr.ip().to_string();
        let port = addr.port();

        let _c = TcpStream::connect(addr)
            .await
            .map(|s| tunnel(s, host, port))
            .unwrap();
    }

    #[tokio::test]
    async fn tunnel_bad_response() {
        let addr = mock_tunnel(Some("foo bar baz hallo")).await;
        let host = addr.ip().to_string();
        let port = addr.port();

        let _c = TcpStream::connect(addr)
            .await
            .map(|s| tunnel(s, host, port))
            .unwrap();
    }
}
