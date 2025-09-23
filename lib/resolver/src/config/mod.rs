#[cfg_attr(target_os = "windows", path = "windows.rs")]
#[cfg_attr(target_family = "unix", path = "unix.rs")]
mod sys;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// https://man7.org/linux/man-pages/man5/resolv.conf.5.html
#[derive(Debug)]
pub struct Config {
    /// server addresses (in host:port form) to use.
    pub servers: Vec<SocketAddr>,
    /// rooted suffixes to append to local name.
    pub search: Vec<String>,
    /// number of dots in name to trigger absolute lookup.
    pub ndots: i32,
    /// wait before giving up on a query, including retries
    pub timeout: Duration,
    /// lost packets before giving up on server.
    pub attempts: u32,
    /// round-robin among servers
    pub rotate: bool,
    /// anything unknown was encountered.
    pub unknown_opt: bool,
    /// OpenBSD top-level database "lookup" order
    #[cfg(target_os = "openbsd")]
    pub lookup: Vec<String>,
    /// time of resolv.conf modification
    #[allow(dead_code)]
    pub mtime: SystemTime,
    /// use sequential A and AAAA queries instead of parallel queries.
    pub single_request: bool,
    /// force usage of TCP for DNS resolutions
    pub use_tcp: bool,
    /// add AD flag to queries
    pub trust_ad: bool,
    /// do not check for config file updates
    pub no_reload: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            servers: default_nameservers(),
            search: vec![],
            ndots: 1,
            timeout: Duration::from_secs(5),
            attempts: 2,

            rotate: false,
            unknown_opt: false,
            #[cfg(target_os = "openbsd")]
            lookup: vec![],
            mtime: UNIX_EPOCH,
            single_request: false,
            use_tcp: false,
            trust_ad: false,
            no_reload: false,
        }
    }
}

/// default name servers to use in the absence of DNS configurations
fn default_nameservers() -> Vec<SocketAddr> {
    vec![
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 53),
        SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 53),
    ]
}
