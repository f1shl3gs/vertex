use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use configurable::configurable_component;
use event::{Metric, tags};
use framework::Source;
use framework::config::{Output, SourceConfig, SourceContext, default_interval};
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use resolver::{Hosts, RecordClass, RecordData, RecordType, Resolver};

const fn default_timeout() -> Duration {
    Duration::from_secs(2)
}

fn default_leases_path() -> PathBuf {
    PathBuf::from("/var/lib/misc/dnsmasq.leases")
}

#[configurable_component(source, name = "dnsmasq")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Dnsmasq host:port addresses
    #[serde(default)]
    name_servers: Vec<SocketAddr>,

    /// Path to the dnsmasq leases file, by default it is `/var/lib/misc/dnsmasq.leases`
    #[serde(default = "default_leases_path")]
    leases_path: PathBuf,

    /// Expose dnsmasq leases as metrics (high cardinality)
    #[serde(default)]
    expose_leases: bool,

    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// Timeout for the TCP/UDP socket
    #[serde(default = "default_timeout", with = "humanize::duration::serde")]
    timeout: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "dnsmasq")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let mut config = resolver::Config {
            timeout: self.timeout,
            ..Default::default()
        };
        if !self.name_servers.is_empty() {
            config.servers = self.name_servers.clone();
        }

        let resolver = Resolver::new(config, Hosts::default());

        let interval = self.interval;
        let leases_path = if self.expose_leases {
            Some(self.leases_path.clone())
        } else {
            None
        };
        let mut shutdown = cx.shutdown;
        let mut output = cx.output;

        Ok(Box::pin(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                let start = Instant::now();
                let result = tokio::select! {
                    _ = &mut shutdown => break,
                    result = gather(&resolver, leases_path.as_ref()) => result
                };

                let elapsed = start.elapsed();
                let up = result.is_ok();

                let mut metrics = result.unwrap_or_default();
                metrics.extend([
                    Metric::gauge("dnsmasq_up", "Whether the dnsmasq query successful", up),
                    Metric::gauge(
                        "dnsmasq_scrape_duration_seconds",
                        "query dnsmasq time in seconds",
                        elapsed,
                    ),
                ]);

                if let Err(_err) = output.send(metrics).await {
                    break;
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

const QUESTIONS: [&str; 7] = [
    "cachesize.bind.",
    "insertions.bind.",
    "evictions.bind.",
    "misses.bind.",
    "hits.bind.",
    "auth.bind.",
    "servers.bind.",
];

async fn gather(
    resolver: &Resolver,
    leases_path: Option<&PathBuf>,
) -> std::io::Result<Vec<Metric>> {
    let mut tasks = FuturesUnordered::from_iter(QUESTIONS.into_iter().map(|question| async move {
        (
            question,
            resolver
                .lookup(question, RecordType::TXT, RecordClass::CHAOS)
                .await,
        )
    }));

    let mut metrics = vec![];
    while let Some((question, result)) = tasks.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(err) => {
                debug!(message = "lookup failed", ?err, question);
                continue;
            }
        };

        if question == "servers.bind." {
            for answer in msg.answers {
                let RecordData::TXT(txt) = answer.data else {
                    continue;
                };

                for data in txt {
                    let str = String::from_utf8_lossy(&data);
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
            }

            continue;
        }

        let Some(answer) = msg.answers.first() else {
            continue;
        };

        let RecordData::TXT(fields) = &answer.data else {
            continue;
        };

        let Some(data) = fields.first() else {
            continue;
        };

        // first byte is the length delimiter
        let value = match String::from_utf8_lossy(data).parse::<f64>() {
            Ok(value) => value,
            Err(_) => continue,
        };

        match question {
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

    if let Some(leases_path) = leases_path {
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
                    %err,
                );
            }
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

fn load_lease_file(path: &PathBuf) -> std::io::Result<Vec<Lease>> {
    let file = std::fs::File::open(path)?;
    let mut lines = BufReader::new(file).lines();

    let mut leases = vec![];
    while let Some(Ok(line)) = lines.next() {
        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
        if parts.len() != 5 {
            continue;
        }

        let Ok(expiry) = parts[0].parse() else {
            warn!(message = "parse lease line failed", line);

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
