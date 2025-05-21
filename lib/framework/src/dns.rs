use std::net::SocketAddr;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::future::BoxFuture;
use hyper_util::client::legacy::connect::dns::Name;
use resolver::RecordData;
use thiserror::Error;
use tower::Service;

pub struct LookupIp(::resolver::LookupIntoIter);

impl Iterator for LookupIp {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.0.next() {
                Some(record) => match record.data {
                    RecordData::A(addr) => return Some(SocketAddr::from((addr, 0))),
                    RecordData::AAAA(addr) => return Some(SocketAddr::from((addr, 0))),
                    _ => continue,
                },
                None => return None,
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum DnsError {
    #[error("resolve error: {0:?}")]
    Resolve(::resolver::Error),
}

#[derive(Clone, Debug)]
pub struct Resolver(Arc<::resolver::Resolver>);

impl Resolver {
    /// Create a new Async resolver
    #[allow(clippy::new_without_default)]
    pub fn new() -> Resolver {
        let inner = ::resolver::Resolver::with_defaults().unwrap();

        Resolver(Arc::new(inner))
    }

    pub async fn lookup_ip(&self, name: &str) -> Result<LookupIp, DnsError> {
        let msg = self.0.lookup_ipv4(name).await.map_err(DnsError::Resolve)?;

        Ok(LookupIp(msg.into_iter()))
    }
}

impl Service<Name> for Resolver {
    type Response = LookupIp;
    type Error = DnsError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, name: Name) -> Self::Future {
        let resolver = self.clone();

        Box::pin(async move { resolver.lookup_ip(name.as_str()).await })
    }
}

#[cfg(test)]
mod tests {
    use super::Resolver;

    async fn resolve(name: &str) -> bool {
        let resolver = Resolver::new();
        resolver.lookup_ip(name).await.is_ok()
    }

    #[tokio::test]
    async fn resolve_example() {
        assert!(resolve("example.com").await);
    }

    #[tokio::test]
    async fn resolve_localhost() {
        assert!(resolve("localhost").await);
    }

    #[tokio::test]
    async fn resolve_ipv4() {
        assert!(resolve("10.0.4.0").await);
    }

    #[tokio::test]
    async fn resolve_ipv6() {
        assert!(resolve("::1").await);
    }
}
