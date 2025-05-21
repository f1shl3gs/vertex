mod hosts;
#[cfg_attr(target_os = "linux", path = "unix.rs")]
mod sys;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::{Duration, SystemTime};

pub use hosts::Hosts;

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

/// default name servers to use in the absence of DNS configurations
fn default_nameservers() -> Vec<SocketAddr> {
    vec![
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 53),
        SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 53),
    ]
}

fn ensure_rooted(s: &str) -> String {
    if !s.is_empty() && s.ends_with('.') {
        return s.to_string();
    }

    s.to_string() + "."
}

fn default_search_with(data: &[u8]) -> Vec<String> {
    let Some(pos) = data.iter().position(|ch| *ch == b'.') else {
        return vec![];
    };

    if pos < data.len() - 1 {
        vec![ensure_rooted(unsafe {
            std::str::from_utf8_unchecked(&data[pos + 1..])
        })]
    } else {
        vec![]
    }
}

fn default_search() -> Vec<String> {
    let max_len = unsafe { libc::sysconf(libc::_SC_HOST_NAME_MAX) };
    if max_len == -1 {
        return vec![];
    }

    // This buffer is far larger than what most systems will ever allow, e.g.
    // linux uses 64 via _SC_HOST_NAME_MAX even though POSIX says the size
    // must be at least _POSIX_HOST_NAME_MAX(255), but other systems can be
    // larger, so we just use a sufficiently sized buffer so we can defer
    // a heap allocation until the last possible moment.
    let mut buf = vec![0u8; max_len as usize];

    let size = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.capacity()) };
    if size == -1 {
        return vec![];
    }

    let Some(pos) = buf.iter().position(|ch| *ch == 0) else {
        return vec![];
    };

    if cfg!(test) {
        default_search_with(b"host.domain.local")
    } else {
        default_search_with(&buf[..pos])
    }
}
