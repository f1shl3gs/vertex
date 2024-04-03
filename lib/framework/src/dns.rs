use std::net::SocketAddr;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::future::BoxFuture;
use hickory_resolver::lookup::{Lookup, LookupIntoIter};
use hickory_resolver::proto::op::Query;
use hickory_resolver::proto::rr::rdata::A;
use hickory_resolver::proto::rr::RData;
use hickory_resolver::{system_conf, TokioAsyncResolver};
use hyper::client::connect::dns::Name;
use thiserror::Error;
use tower::Service;

pub struct LookupIp(LookupIntoIter);

impl Iterator for LookupIp {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|rdata| match rdata {
            RData::A(a) => SocketAddr::from((a.0, 0)),
            RData::AAAA(aaaa) => SocketAddr::from((aaaa.0, 0)),
            _ => panic!("invalid resolve response"),
        })
    }
}

#[derive(Debug, Error)]
pub enum DnsError {
    #[error(transparent)]
    Resolve(hickory_resolver::error::ResolveError),
}

#[derive(Clone, Debug)]
pub struct Resolver(Arc<TokioAsyncResolver>);

impl Resolver {
    /// Create a new Async resolver
    #[allow(clippy::new_without_default)]
    pub fn new() -> Resolver {
        let (config, opts) =
            system_conf::read_system_conf().expect("Read system config of DNS failed");
        let inner = TokioAsyncResolver::tokio(config, opts);

        Resolver(Arc::new(inner))
    }

    pub async fn lookup_ip(&self, name: &str) -> Result<LookupIp, DnsError> {
        let lookup = if name == "localhost" {
            // Not all operating systems support `localhost` as IPv6 `::1`, so
            // we resolving it to it's IPv4 value.
            Lookup::from_rdata(Query::default(), RData::A(A::new(127, 0, 0, 1)))
        } else {
            let inner = self.0.lookup_ip(name).await.map_err(DnsError::Resolve)?;
            Lookup::from(inner)
        };

        Ok(LookupIp(lookup.into_iter()))
    }
}

impl Service<Name> for Resolver {
    type Response = LookupIp;
    type Error = DnsError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
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
