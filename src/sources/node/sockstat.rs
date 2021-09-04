/// Exposes various statistics from /proc/net/sockstat and /proc/net/sockstat6
///

use crate::event::Metric;
use crate::sources::node::errors::Error;
use crate::sources::node::read_to_string;

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, ()> {
    todo!()
}

/// NetSockstatProtocol contains statistics about a given socket protocol.
/// Option fields indicate that the value may or may not be present on
/// any given protocol
struct NetSockstatProtocol {
    protocol: String,
    inuse: i32,
    orphan: Option<i32>,
    tw: Option<i32>,
    alloc: Option<i32>,
    mem: Option<i32>,
    memory: Option<i32>,
}

async fn read_sockestat(path: &str) -> Result<NetSockstatProtocol, Error> {
    // This file is small and can be read with one syscall
    let content = read_to_string(path).await?;

    todo!()
}

fn parse_sockestat(content: &str) -> Result<NetSockstatProtocol, Error> {
    todo!()
}