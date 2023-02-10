use std::time::Duration;

use configurable::configurable_component;
use event::Metric;
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::{
    config::{default_interval, DataType, Output, SourceConfig, SourceContext},
    Source,
};
use futures::StreamExt;
use rsntp;
use tokio_stream::wrappers::IntervalStream;

#[configurable_component(source, name = "ntp")]
#[derive(Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct NtpConfig {
    /// NTP servers to use.
    #[configurable(required, format = "hostname", example = "pool.ntp.org")]
    pub pools: Vec<String>,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// The NTP client query timeout
    #[serde(default = "default_timeout")]
    #[serde(with = "humanize::duration::serde")]
    pub timeout: Duration,
}

const fn default_timeout() -> Duration {
    Duration::from_secs(10)
}

#[async_trait::async_trait]
#[typetag::serde(name = "ntp")]
impl SourceConfig for NtpConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let ntp = Ntp {
            interval: self.interval,
            timeout: self.timeout,
            pools: self.pools.clone(),
            pick_state: 0,
        };

        Ok(Box::pin(ntp.run(cx.shutdown, cx.output)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }
}

struct Ntp {
    interval: Duration,
    timeout: Duration,
    pools: Vec<String>,

    pick_state: usize,
}

impl Ntp {
    fn pick_one(&mut self) -> String {
        self.pick_state += 1;

        let index = self.pick_state % self.pools.len();

        self.pools[index].clone()
    }

    async fn run(mut self, shutdown: ShutdownSignal, mut out: Pipeline) -> Result<(), ()> {
        let interval = tokio::time::interval(self.interval);
        let mut ticker = IntervalStream::new(interval).take_until(shutdown);

        let mut client = rsntp::AsyncSntpClient::new();
        client.set_timeout(self.timeout);

        while let Some(_ts) = ticker.next().await {
            let addr = self.pick_one();

            match client.synchronize(addr).await {
                Ok(result) => {
                    let timestamp = Some(chrono::Utc::now());
                    let clock_offset = result.clock_offset().as_secs_f64();
                    let rtt = result.round_trip_delay().as_secs_f64();
                    let leap = result.leap_indicator() as u8 as f64;

                    let mut metrics = vec![
                        Metric::gauge("ntp_stratum", "NTPD stratum", result.stratum()),
                        Metric::gauge("ntp_leap", "NTPD leap second indicator, 2 bits", leap),
                        Metric::gauge("ntp_rtt_seconds", "RTT to NTPD", rtt),
                        Metric::gauge(
                            "ntp_offset_seconds",
                            "ClockOffset between NTP and local clock",
                            clock_offset,
                        ),
                        // TODO: reference_timestamp_seconds
                        // TODO: root_delay_seconds
                        // TODO: root_dispersion_seconds
                        // TODO: sanity
                    ];

                    metrics.iter_mut().for_each(|m| m.timestamp = timestamp);

                    if let Err(err) = out.send(metrics).await {
                        error!(
                            message = "Error sending ntp metrics",
                            %err
                        );

                        return Err(());
                    }
                }

                Err(err) => {
                    warn!("Synchronize failed, {}", err);
                }
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
        crate::testing::test_generate_config::<NtpConfig>()
    }
}
