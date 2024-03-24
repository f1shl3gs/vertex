use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::future::BoxFuture;
use hickory_resolver::{system_conf, TokioAsyncResolver};
use hyper::client::connect::dns::Name;
use thiserror::Error;
use tower::Service;

pub struct LookupIp(std::vec::IntoIter<SocketAddr>);

impl Iterator for LookupIp {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
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
        // We need to add port with the name so that `to_socket_addrs`
        // resolves it properly. We will be discarding the port afterwards.
        //
        // Any port will do, but `9` is a well defined port for discarding
        // packets.
        let dummy_port = 9;
        // https://tools.ietf.org/html/rfc6761#section-6.3
        if name == "localhost" {
            // Not all operating systems support `localhost` as IPv6 `::1`, so
            // we resolving it to it's IPv4 value.
            Ok(LookupIp(
                vec![SocketAddr::new(Ipv4Addr::LOCALHOST.into(), dummy_port)].into_iter(),
            ))
        } else {
            let addrs = self
                .0
                .lookup_ip(name)
                .await
                .map_err(DnsError::Resolve)?
                .iter()
                .map(|addr| SocketAddr::new(addr, 0))
                .collect::<Vec<_>>();

            Ok(LookupIp(addrs.into_iter()))
        }
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
