use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::portpicker::pick_unused_port;

pub fn next_addr_for_ip(ip: IpAddr) -> SocketAddr {
    let port = pick_unused_port(ip);
    SocketAddr::new(ip, port)
}

pub fn next_addr() -> SocketAddr {
    next_addr_for_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
}
