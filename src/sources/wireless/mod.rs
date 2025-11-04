mod nl80211;

use std::io::ErrorKind;
use std::time::Duration;

use configurable::configurable_component;
use event::{Metric, tags};
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::{Pipeline, ShutdownSignal, Source};

use nl80211::{Client, Error};

#[configurable_component(source, name = "wireless")]
struct Config {
    /// Duration between each collecting.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "wireless")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        Ok(Box::pin(run(self.interval, cx.output, cx.shutdown)))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

async fn run(
    interval: Duration,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let timeout = Duration::from_secs(10);
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            _ = &mut shutdown => break,
        }

        match tokio::time::timeout(timeout, collect()).await {
            Ok(Ok(metrics)) => {
                if let Err(_err) = output.send(metrics).await {
                    break;
                }
            }
            Ok(Err(err)) => {
                warn!(message = "collect wifi metrics failed", %err);
            }
            Err(_err) => {
                warn!(message = "collect wifi metrics timeout", ?timeout);
            }
        }
    }

    Ok(())
}

async fn collect() -> Result<Vec<Metric>, Error> {
    let mut client = Client::connect().await.inspect_err(|err| match err {
        Error::Io(err) if err.kind() == ErrorKind::NotFound => {
            debug!(message = "WiFi collector got permission denied when accessing metrics");
        }
        _ => {
            debug!(message = "WiFi collector metrics might not available for this system");
        }
    })?;

    let interfaces = client.interfaces().await?;
    let mut metrics = Vec::with_capacity(interfaces.len() * 14);

    for interface in interfaces {
        // some virtual devices have no "name" and should be skipped
        if interface.name.is_empty() {
            continue;
        }

        debug!(
            message = "probing wifi device",
            interface = interface.name,
            r#type = interface.typ.as_str()
        );

        metrics.push(Metric::gauge_with_tags(
            "wifi_interface_frequency_hertz",
            "The current frequency a WiFi interface is operating at, in hertz",
            interface.frequency as f64 * 1000.0 * 1000.0,
            tags!(
                "device" => interface.name.clone(),
            ),
        ));

        match client.bss(&interface).await {
            Ok(bss) => {
                metrics.push(Metric::gauge_with_tags(
                    "wifi_station_info",
                    "Labeled WiFi interface station information as provided by the operating system",
                    1,
                    tags!(
                        "device" => interface.name.clone(),
                        "bssid" => mac_address(&bss.mac),
                        "ssid" => bss.ssid.clone(),
                        "mode" => match bss.status {
                            0 | 1 => "client",
                            3 => "ad-hoc",
                            _ => "unknown"
                        }
                    ),
                ));
            }
            Err(err) => {
                if matches!(err, Error::NotExists) {
                    debug!(
                        message = "BSS information not found",
                        interface = interface.name
                    );
                } else {
                    warn!(
                        message = "collect bss failed",
                        interface = interface.name,
                        ?err
                    );

                    continue;
                }
            }
        };

        match client.station_info(&interface).await {
            Ok(infos) => {
                for info in infos {
                    let tags = tags!(
                        "device" => interface.name.clone(),
                        "mac" => mac_address(&info.mac),
                    );

                    metrics.extend([

                        Metric::sum_with_tags(
                            "wifi_station_connected_seconds_total",
                            "The total number of seconds a station has been connected to an access point",
                            info.connected,
                            tags.clone()
                        ),
                        Metric::gauge_with_tags(
                            "wifi_station_inactive_seconds",
                            "The number of seconds since any wireless activity has occurred on a station",
                            info.inactive,
                            tags.clone()
                        ),
                        Metric::gauge_with_tags(
                            "wifi_station_receive_bits_per_second",
                            "The current WiFi receive bitrate of a station, in bits per second",
                            info.receive_bitrate,
                            tags.clone()
                        ),
                        Metric::gauge_with_tags(
                            "wifi_station_transmit_bits_per_second",
                            "The current WiFi transmit bitrate of a station, in bits per second",
                            info.transmit_bitrate,
                            tags.clone()
                        ),
                        Metric::sum_with_tags(
                            "wifi_station_receive_bytes_total",
                            "The total number of bytes received by a WiFi station",
                            info.received_bytes,
                            tags.clone()
                        ),
                        Metric::sum_with_tags(
                            "wifi_station_transmit_bytes_total",
                            "The total number of bytes transmitted by a WiFi station",
                            info.transmitted_bytes,
                            tags.clone()
                        ),
                        Metric::gauge_with_tags(
                            "wifi_station_signal_dbm",
                            "The current WiFi signal strength, in decibel-milliwatts (dBm)",
                            info.signal,
                            tags.clone()
                        ),
                        Metric::sum_with_tags(
                            "wifi_station_transmit_retries_total",
                            "The total number of times a station has had to retry while sending a packet",
                            info.transmit_retries,
                            tags.clone()
                        ),
                        Metric::sum_with_tags(
                            "wifi_station_transmit_failed_total",
                            "The total number of times a station has failed to send a packet",
                            info.transmit_failed,
                            tags.clone()
                        ),
                        Metric::sum_with_tags(
                            "wifi_station_beacon_loss_total",
                            "The total number of times a station has detected a beacon loss",
                            info.beacon_loss,
                            tags.clone()
                        ),
                        Metric::sum_with_tags(
                            "wifi_station_transmitted_packets_total",
                            "The total number of packets transmitted by a station",
                            info.transmitted_packets,
                            tags.clone()
                        ),
                        Metric::sum_with_tags(
                            "wifi_station_received_packets_total",
                            "The total number of packets received by a station",
                            info.received_packets,
                            tags
                        )
                    ]);
                }
            }
            Err(err) => {
                if matches!(err, Error::NotExists) {
                    debug!(
                        message = "station information not found",
                        interface = interface.name
                    );
                } else {
                    warn!(
                        message = "collect station information failed",
                        interface = interface.name,
                        ?err
                    );
                }
            }
        }
    }

    Ok(metrics)
}

fn mac_address(data: &[u8]) -> String {
    data.iter()
        .take(6)
        .map(|v| format!("{:02x}", v))
        .collect::<Vec<_>>()
        .join(":")
}
