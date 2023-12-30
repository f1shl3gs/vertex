use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::{Add, Sub};
use std::time::{Duration, Instant};

use chrono::{DurationRound, Utc};
use configurable::configurable_component;
use event::Metric;
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::{
    config::{default_interval, DataType, Output, SourceConfig, SourceContext},
    Source,
};
use ntp::Client;

const fn default_timeout() -> Duration {
    Duration::from_secs(10)
}

const fn default_bind() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)
}

/// This source checks the drift of that node's clock against a given NTP server or servers.
#[configurable_component(source, name = "ntp")]
#[derive(Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// SocketAddr which UdpSocket is going to bind.
    #[serde(default = "default_bind")]
    bind: SocketAddr,

    /// NTP servers to use.
    #[configurable(required, format = "hostname", example = "pool.ntp.org")]
    pools: Vec<String>,

    /// The NTP client query timeout
    #[serde(default = "default_timeout")]
    #[serde(with = "humanize::duration::serde")]
    timeout: Duration,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "ntp")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let mut pools = Vec::with_capacity(self.pools.len());
        for pool in &self.pools {
            let pool = match pool.split_once(':') {
                Some((_host, port)) => {
                    // validate pool
                    port.parse::<u16>()
                        .map_err(|_err| format!("invalid pool {pool}"))?;
                    pool.to_string()
                }
                // port is not specified
                None => format!("{pool}:123"),
            };

            pools.push(pool);
        }

        let ntp = Ntp {
            client: Client::new(self.bind),
            pools,
            pick_state: 0,
        };

        Ok(Box::pin(ntp.run(
            cx.shutdown,
            cx.output,
            self.interval,
            self.timeout,
        )))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }
}

struct Ntp {
    client: Client,

    pools: Vec<String>,
    pick_state: usize,
}

impl Ntp {
    fn pick_one(&mut self) -> String {
        self.pick_state += 1;

        let index = self.pick_state % self.pools.len();

        self.pools[index].clone()
    }

    async fn run(
        mut self,
        mut shutdown: ShutdownSignal,
        mut out: Pipeline,
        interval: Duration,
        timeout: Duration,
    ) -> Result<(), ()> {
        let max_distance = chrono::Duration::nanoseconds(3466080000); // 3.46608s
        let mut ticker = tokio::time::interval(interval);
        let mut max_err = chrono::Duration::milliseconds(1);
        let mut leap_midnight = Utc::now(); // dummy default value
        let day = chrono::Duration::days(1);

        loop {
            tokio::select! {
                biased;

                _ = &mut shutdown => break,
                _ = ticker.tick() => {}
            }

            let addr = self.pick_one();
            let start = Instant::now();
            let result = tokio::time::timeout(timeout, self.client.query(&addr)).await;
            let elapsed = start.elapsed();

            let metrics = match result {
                Ok(Ok(resp)) => {
                    // LeapAddSecond(1) indicates the last minute of the day has 61 seconds.
                    // LeapDelSecond(2) indicates the last minute of the day has 59 seconds.
                    if resp.leap == 1 || resp.leap == 2 {
                        // state of leap_midnight is cached as leap flag is dropped right after midnigh
                        leap_midnight = resp.time.duration_trunc(day).unwrap() + day;
                    }
                    if leap_midnight.sub(day) < resp.time && resp.time < leap_midnight.add(day) {
                        // tolerate leap smearing
                        max_err = max_err + chrono::Duration::seconds(1);
                    }

                    let sanity = resp.validate().is_ok()
                        && resp.root_distance <= max_distance
                        && resp.min_err <= max_err;

                    vec![
                        Metric::gauge("ntp_stratum", "NTPD stratum", resp.stratum),
                        Metric::gauge("ntp_leap", "NTPD leap second indicator, 2 bits", resp.leap),
                        Metric::gauge(
                            "ntp_rtt_seconds",
                            "RTT to NTPD",
                            resp.rtt.num_nanoseconds().unwrap() as f64 / 1_000_000_000.0,
                        ),
                        Metric::gauge(
                            "ntp_offset_seconds",
                            "ClockOffset between NTP and local clock",
                            resp.clock_offset.num_nanoseconds().unwrap() as f64 / 1_000_000_000.0,
                        ),
                        Metric::gauge(
                            "ntp_reference_timestamp_seconds",
                            "NTPD ReferenceTime, UNIX timestamp",
                            resp.reference_time.timestamp_nanos_opt().unwrap() as f64
                                / 1_000_000_000.0,
                        ),
                        Metric::gauge(
                            "ntp_root_delay_seconds",
                            "NTPD RootDelay",
                            resp.root_delay.num_nanoseconds().unwrap() as f64 / 1_000_000_000.0,
                        ),
                        Metric::gauge(
                            "ntp_dispersion_seconds",
                            "NTPD RootDispersion",
                            resp.root_dispersion.num_nanoseconds().unwrap() as f64
                                / 1_000_000_000.0,
                        ),
                        Metric::gauge(
                            "ntp_sanity",
                            "NTPD sanity according to RFC5905 heuristics and configured limits",
                            sanity,
                        ),
                        Metric::gauge("ntp_up", "NTP query health", 1),
                        Metric::gauge(
                            "ntp_scrape_duration_seconds",
                            "Duration of NTP query",
                            elapsed,
                        ),
                    ]
                }
                Ok(Err(err)) => {
                    warn!(message = "ntp query failed", server = addr, ?err);

                    vec![
                        Metric::gauge("ntp_up", "NTP query health", 0),
                        Metric::gauge(
                            "ntp_scrape_duration_seconds",
                            "Duration of NTP query",
                            elapsed,
                        ),
                    ]
                }
                Err(_) => {
                    warn!(message = "ntp query timeout", server = addr, ?timeout);

                    vec![
                        Metric::gauge("ntp_up", "NTP query health", 0),
                        Metric::gauge(
                            "ntp_scrape_duration_seconds",
                            "Duration of NTP query",
                            elapsed,
                        ),
                    ]
                }
            };

            if let Err(err) = out.send(metrics).await {
                error!(
                    message = "Error sending ntp metrics",
                    %err
                );

                return Err(());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
