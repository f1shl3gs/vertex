mod dns;

use std::io::{BufRead, BufReader, Cursor};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::AtomicU16;
use std::time::Duration;

use configurable::configurable_component;
use event::{Metric, tags};
use framework::Source;
use framework::config::{Output, SourceConfig, SourceContext, default_interval};
use tokio::net::UdpSocket;

use dns::Encodable;

const fn default_endpoint() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 53)
}

fn default_leases_path() -> PathBuf {
    PathBuf::from("/var/lib/misc/dnsmasq.leases")
}

#[configurable_component(source, name = "dnsmasq")]
struct Config {
    #[serde(default = "default_endpoint")]
    endpoint: SocketAddr,

    #[serde(default = "default_leases_path")]
    leases_path: PathBuf,

    #[serde(default)]
    expose_leases: bool,

    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "dnsmasq")]
impl SourceConfig for Config {
    // Good news, we have an async resolver, aka hickory-resolver
    // Bad news, it doesn't support lookup by Query, while we need to set the class to
    //   DNSClass::CH (Chaos)
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let interval = self.interval;
        let name_server = self.endpoint;
        let leases_path = self.leases_path.clone();
        let mut shutdown = cx.shutdown;
        let mut output = cx.output;

        Ok(Box::pin(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                match gather(name_server, &leases_path).await {
                    Ok(metrics) => {
                        if let Err(_err) = output.send(metrics).await {
                            break;
                        }
                    }
                    Err(err) => {
                        warn!(message = "fetch dnsmasq metrics failed", ?err);
                    }
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::metrics()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

const QUESTIONS: [&str; 7] = [
    "cachesize.bind.",
    "insertions.bind.",
    "evictions.bind.",
    "misses.bind.",
    "hits.bind.",
    "auth.bind.",
    "servers.bind.",
];

static ID_GEN: AtomicU16 = AtomicU16::new(1);

fn next_id() -> u16 {
    ID_GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

async fn gather(name_server: SocketAddr, leases_path: &PathBuf) -> Result<Vec<Metric>, Error> {
    let socket =
        UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0)).await?;
    socket.connect(&name_server).await?;

    let mut buf = Cursor::new([0u8; 512]);
    let mut metrics = vec![];
    for question in &QUESTIONS {
        let msg = dns::Message {
            header: dns::Header {
                id: next_id(),
                recursion_desired: true,
                ..Default::default()
            },
            questions: vec![dns::Question {
                name: question.to_string(),
                typ: 16,  // TXT
                class: 3, // CHAOS
            }],
            ..Default::default()
        };

        buf.set_position(0);
        msg.encode(&mut buf).unwrap();
        let len = buf.position() as usize;
        socket.send(&buf.get_mut()[..len]).await?;

        buf.set_position(0);
        socket.recv(buf.get_mut()).await?;
        let msg = dns::Message::decode(&mut buf).unwrap();

        if *question == "servers.bind." {
            for answer in msg.answers {
                let str = String::from_utf8_lossy(&answer.data);
                let arr = str.split_ascii_whitespace().collect::<Vec<_>>();
                if arr.len() != 3 {
                    continue;
                }

                let server = arr[0];
                let Ok(queries) = arr[1].parse::<f64>() else {
                    continue;
                };
                let Ok(queries_failed) = arr[2].parse::<f64>() else {
                    continue;
                };

                metrics.extend([
                    Metric::gauge_with_tags(
                        "dnsmasq_servers_queries",
                        "DNS queries on upstream server",
                        queries,
                        tags! {"server" => server},
                    ),
                    Metric::gauge_with_tags(
                        "dnsmasq_servers_queries_failed",
                        "DNS queries failed on upstream server",
                        queries_failed,
                        tags! {"server" => server},
                    ),
                ]);
            }

            continue;
        }

        let Some(answer) = msg.answers.first() else {
            continue;
        };

        let value = match String::from_utf8_lossy(&answer.data[1..]).parse::<f64>() {
            Ok(value) => value,
            Err(_) => continue,
        };

        match *question {
            "cachesize.bind." => metrics.push(Metric::gauge(
                "dnsmasq_cache_size",
                "configured size of the DNS cache",
                value,
            )),
            "insertions.bind." => metrics.push(Metric::gauge(
                "dnsmasq_insertions",
                "DNS cache insertions",
                value,
            )),
            "evictions.bind." => metrics.push(Metric::gauge(
                "dnsmasq_evictions",
                "DNS cache evictions: numbers of entries which replaced an unexpired cache entry",
                value,
            )),
            "misses.bind." => metrics.push(Metric::sum(
                "dnsmasq_misses",
                "DNS cache misses: queries which had to be forwarded",
                value,
            )),
            "hits.bind." => metrics.push(Metric::sum(
                "dnsmasq_hits",
                "DNS queries answered locally (cache hits)",
                value,
            )),
            "auth.bind." => metrics.push(Metric::sum(
                "dnsmasq_auths",
                "DNS queries for authoritative zones",
                value,
            )),
            _ => continue,
        }
    }

    match load_lease_file(leases_path) {
        Ok(leases) => {
            metrics.push(Metric::gauge(
                "dnsmasq_leases",
                "Number of DHCP leases handed out",
                leases.len() as f64,
            ));

            for lease in leases {
                metrics.push(Metric::gauge_with_tags(
                    "dnsmasq_lease_expiry",
                    "Expiry time for active DHCP leases",
                    lease.expiry,
                    tags!(
                        "mac" => lease.mac,
                        "ip" => lease.ip,
                        "computer" => lease.computer,
                        "client_id" => lease.client_id,
                    ),
                ))
            }
        }
        Err(err) => {
            warn!(
                message = "load lease file failed",
                path = ?leases_path,
                ?err,
            );
        }
    }

    Ok(metrics)
}

struct Lease {
    expiry: u64,
    mac: String,
    ip: String,
    computer: String,
    client_id: String,
}

fn load_lease_file(path: &PathBuf) -> Result<Vec<Lease>, Error> {
    let file = std::fs::File::open(path).map_err(Error::Io)?;
    let mut lines = BufReader::new(file).lines();

    let mut leases = vec![];
    while let Some(Ok(line)) = lines.next() {
        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
        if parts.len() != 5 {
            continue;
        }

        let Ok(expiry) = parts[0].parse() else {
            warn!(message = "parse lease line failed", line,);

            continue;
        };

        leases.push(Lease {
            expiry,
            mac: parts[1].to_string(),
            ip: parts[2].to_string(),
            computer: parts[3].to_string(),
            client_id: parts[4].to_string(),
        });
    }

    Ok(leases)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
