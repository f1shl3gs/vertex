use std::collections::HashMap;
use std::io;
use std::io::ErrorKind;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use configurable::configurable_component;
use framework::Extension;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::observe::{Endpoint, Observer, run};
use value::value;

fn default_proc() -> PathBuf {
    PathBuf::from("/proc")
}

const fn default_interval() -> Duration {
    Duration::from_secs(10)
}

#[configurable_component(extension, name = "host_observer")]
struct Config {
    /// Absolute Path to the `/proc`
    #[serde(default = "default_proc")]
    proc_path: PathBuf,

    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "host_observer")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        let observer = Observer::register(cx.key);
        let root = self.proc_path.clone();

        Ok(Box::pin(run(
            observer,
            self.interval,
            cx.shutdown,
            async move || list_endpoints(&root).await,
        )))
    }
}

async fn list_endpoints(root: &PathBuf) -> crate::Result<Vec<Endpoint>> {
    let infos = netstat(root)?;

    let endpoints = infos
        .into_iter()
        .map(|info| {
            let ConnectionInfo {
                pid,
                name,
                cmdline,
                protocol,
                addr,
            } = info;
            let port = addr.port();
            let is_ipv6 = addr.is_ipv6();
            let protocol = match protocol {
                Protocol::Tcp => "tcp",
                Protocol::Udp => "udp",
            };

            Endpoint {
                id: format!("{}:{}:{}@{}", protocol, addr.ip(), addr.port(), pid),
                typ: "host".into(),
                target: info.addr.to_string(),
                details: value!({
                    "name": name,
                    "port": port,
                    "pid": pid,
                    "cmdline": cmdline,
                    "is_ipv6": is_ipv6,
                    "protocol": protocol,
                }),
            }
        })
        .collect::<Vec<_>>();

    Ok(endpoints)
}

fn parse_socket_addr(input: &str) -> io::Result<SocketAddr> {
    // addr looks like EF58A8C0:0044
    let (addr, port) = input.split_once(':').unwrap();
    let port = u16::from_str_radix(port, 16).unwrap();

    if addr.len() == 8 {
        let octets = u32::from_str_radix(addr, 16).unwrap().to_le_bytes();
        Ok((Ipv4Addr::from(octets), port).into())
    } else if addr.len() == 32 {
        let a = u32::from_str_radix(&addr[..8], 16).unwrap();
        let b = u32::from_str_radix(&addr[8..16], 16).unwrap();
        let c = u32::from_str_radix(&addr[16..24], 16).unwrap();
        let d = u32::from_str_radix(&addr[24..32], 16).unwrap();

        let mut octets = [0u8; 16];
        octets[..4].copy_from_slice(&a.to_le_bytes());
        octets[4..8].copy_from_slice(&b.to_le_bytes());
        octets[8..12].copy_from_slice(&c.to_le_bytes());
        octets[12..16].copy_from_slice(&d.to_le_bytes());

        Ok((Ipv6Addr::from(octets), port).into())
    } else {
        Err(ErrorKind::InvalidInput.into())
    }
}

enum Protocol {
    Tcp,
    Udp,
}

struct ConnectionInfo {
    pid: u32,
    name: String,
    cmdline: String,

    protocol: Protocol,
    addr: SocketAddr,
}

fn parse_listen_inodes(path: PathBuf) -> io::Result<HashMap<String, SocketAddr>> {
    let mut inodes = HashMap::new();

    let content = std::fs::read_to_string(&path)?;
    for line in content.lines().skip(1) {
        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
        if parts.len() < 10 {
            continue;
        }

        // "0A" represent for LISTEN
        if parts[3] != "0A" {
            continue;
        }

        match parse_socket_addr(parts[1]) {
            Ok(addr) => inodes.insert(parts[9].to_string(), addr),
            Err(_) => continue,
        };
    }

    Ok(inodes)
}

/// Get process which listen to an ipv4 TCP socket
///
/// ```text
///  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
///   0: 010017AC:84B7 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 11773951 1 000000002ca9405f 100 0 0 10 0
///```
///
/// See: https://www.kernel.org/doc/Documentation/networking/proc_net_tcp.txt
fn netstat(root: &PathBuf) -> io::Result<Vec<ConnectionInfo>> {
    let mut tcp = parse_listen_inodes(root.join("net/tcp"))?;
    let mut tcp6 = parse_listen_inodes(root.join("net/tcp6"))?;
    let mut udp = parse_listen_inodes(root.join("net/udp"))?;
    let mut udp6 = parse_listen_inodes(root.join("net/udp6"))?;

    let mut infos = Vec::new();
    let dirs = std::fs::read_dir(root)?;
    for entry in dirs.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };

        let fd_path = entry.path().join("fd");
        if !fd_path.is_dir() {
            continue;
        }

        let fd_dirs = match std::fs::read_dir(fd_path) {
            Ok(dirs) => dirs,
            Err(err) => match err.kind() {
                ErrorKind::NotFound | ErrorKind::PermissionDenied | ErrorKind::UnexpectedEof => {
                    continue;
                }
                _ => return Err(err),
            },
        };

        for fd_entry in fd_dirs.flatten() {
            match std::fs::read_link(fd_entry.path()) {
                Ok(path) => {
                    // target path should look like `socket:[13816975]`
                    let path = path.to_string_lossy();
                    let Some(striped) = path.strip_prefix("socket:[") else {
                        continue;
                    };

                    if let Some(key) = striped.strip_suffix(']') {
                        let (addr, protocol) = if let Some(addr) = tcp.remove(key) {
                            (addr, Protocol::Tcp)
                        } else if let Some(addr) = tcp6.remove(key) {
                            (addr, Protocol::Tcp)
                        } else if let Some(addr) = udp.remove(key) {
                            (addr, Protocol::Udp)
                        } else if let Some(addr) = udp6.remove(key) {
                            (addr, Protocol::Udp)
                        } else {
                            continue;
                        };

                        let mut name = std::fs::read_to_string(entry.path().join("comm"))?;
                        let new_len = name.trim_end().len();
                        name.truncate(new_len);

                        let mut cmdline = std::fs::read_to_string(entry.path().join("cmdline"))?;
                        let new_len = cmdline.trim_end_matches('\0').len();
                        cmdline.truncate(new_len);

                        infos.push(ConnectionInfo {
                            pid,
                            name,
                            cmdline,
                            protocol,
                            addr,
                        });

                        if tcp.is_empty() && tcp6.is_empty() && udp.is_empty() && udp6.is_empty() {
                            return Ok(infos);
                        }
                    }
                }
                Err(err) => match err.kind() {
                    ErrorKind::PermissionDenied
                    | ErrorKind::NotFound
                    | ErrorKind::UnexpectedEof => continue,
                    _ => continue,
                },
            };
        }
    }

    Ok(infos)
}
