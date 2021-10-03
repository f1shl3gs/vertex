use chrono::Duration;
use serde::{Deserialize, Serialize};
use crate::{
    config::{
        deserialize_duration,
        serialize_duration,
    }
};
use crate::config::{SourceConfig, SourceContext, DataType};
use crate::sources::Source;
use crate::shutdown::ShutdownSignal;
use crate::pipeline::Pipeline;
use tokio_stream::wrappers::IntervalStream;
use futures::{stream, StreamExt, SinkExt};
use rsntp;
use event::{
    Event,
    Metric,
    MetricValue,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NTPConfig {
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    pub timeout: Duration,

    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    pub interval: Duration,

    pub pools: Vec<String>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "ntp")]
impl SourceConfig for NTPConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let ntp = NTP {
            interval: self.interval.to_std()?,
            timeout: self.timeout.to_std()?,
            pools: self.pools.clone(),
            pick_state: 0,
        };

        Ok(Box::pin(ntp.run(ctx.shutdown, ctx.out)))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "ntp"
    }
}


struct NTP {
    interval: std::time::Duration,
    timeout: std::time::Duration,
    pools: Vec<String>,

    pick_state: usize,
}

impl NTP {
    fn pick_one(&mut self) -> String {
        self.pick_state += 1;

        let index = self.pick_state % self.pools.len();

        self.pools[index].clone()
    }

    async fn run(mut self, shutdown: ShutdownSignal, mut out: Pipeline) -> Result<(), ()> {
        let interval = tokio::time::interval(self.interval);
        let mut ticker = IntervalStream::new(interval)
            .take_until(shutdown);

        let mut client = rsntp::AsyncSntpClient::new();
        client.set_timeout(self.timeout);

        while let Some(now) = ticker.next().await {
            let addr = self.pick_one();

            match client.synchronize(addr).await {
                Ok(result) => {
                    let timestamp = now.elapsed().as_secs() as i64;
                    let clock_offset = result.clock_offset().num_milliseconds() as f64 / 1000.0;
                    let rtt = result.round_trip_delay().num_milliseconds() as f64 / 1000.0;
                    let leap = result.leap_indicator() as u8 as f64;

                    let metrics = vec![
                        Metric {
                            name: "ntp_stratum".into(),
                            description: Some("NTPD stratum".into()),
                            tags: Default::default(),
                            unit: None,
                            timestamp,
                            value: MetricValue::gauge(result.stratum()),
                        },
                        Metric {
                            name: "ntp_leap".into(),
                            description: Some("NTPD leap second indicator, 2 bits".into()),
                            tags: Default::default(),
                            unit: None,
                            timestamp,
                            value: MetricValue::Gauge(leap),
                        },
                        Metric {
                            name: "ntp_rtt_seconds".into(),
                            description: Some("RTT to NTPD".into()),
                            tags: Default::default(),
                            unit: None,
                            timestamp,
                            value: MetricValue::Gauge(rtt),
                        },
                        Metric {
                            name: "ntp_offset_seconds".into(),
                            description: Some("ClockOffset between NTP and local clock".into()),
                            tags: Default::default(),
                            unit: None,
                            timestamp,
                            value: MetricValue::Gauge(clock_offset),
                        },
                        // TODO: reference_timestamp_seconds
                        // TODO: root_delay_seconds
                        // TODO: root_dispersion_seconds
                        // TODO: sanity
                    ];

                    let mut stream = stream::iter(metrics)
                        .map(Event::Metric)
                        .map(Ok);

                    out.send_all(&mut stream).await;
                }

                Err(err) => {
                    warn!("Synchronize failed, {}", err);
                }
            }
        }

        Ok(())
    }
}
