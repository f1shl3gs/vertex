use event::{Event, Metric};
use futures::{stream, StreamExt};
use rsntp;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio_stream::wrappers::IntervalStream;

use crate::config::Output;
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;
use crate::sources::Source;
use crate::{
    config::{
        default_interval, deserialize_duration, serialize_duration, DataType, GenerateConfig,
        SourceConfig, SourceContext,
    },
    register_source_config,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NtpConfig {
    #[serde(default = "default_timeout")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    pub timeout: Duration,

    #[serde(default = "default_interval")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    pub interval: Duration,

    pub pools: Vec<String>,
}

const fn default_timeout() -> Duration {
    Duration::from_secs(10)
}

impl GenerateConfig for NtpConfig {
    fn generate_config() -> String {
        format!(
            r#"
# NTP servers to use.
pools:
- 0.pool.ntp.org
- 1.pool.ntp.org
- 2.pool.ntp.org
- 3.pool.ntp.org

# The query timeout
# timeout: {}s

# The interval between scrapes.
#
# interval: {}s
"#,
            default_timeout().as_secs(),
            default_interval().as_secs()
        )
    }
}

register_source_config!("ntp", NtpConfig);

#[async_trait::async_trait]
#[typetag::serde(name = "ntp")]
impl SourceConfig for NtpConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let ntp = Ntp {
            interval: self.interval,
            timeout: self.timeout,
            pools: self.pools.clone(),
            pick_state: 0,
        };

        Ok(Box::pin(ntp.run(ctx.shutdown, ctx.output)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        "ntp"
    }
}

struct Ntp {
    interval: std::time::Duration,
    timeout: std::time::Duration,
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
                    let clock_offset = result.clock_offset().num_milliseconds() as f64 / 1000.0;
                    let rtt = result.round_trip_delay().num_milliseconds() as f64 / 1000.0;
                    let leap = result.leap_indicator() as u8 as f64;

                    let metrics = vec![
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

                    let mut stream = stream::iter(metrics).map(|mut m| {
                        m.timestamp = timestamp;
                        Event::Metric(m)
                    });

                    if let Err(err) = out.send_all(&mut stream).await {
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
        crate::config::test_generate_config::<NtpConfig>()
    }
}
