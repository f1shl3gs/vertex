use chrono::{DateTime, TimeZone, Utc};
use futures::{FutureExt, SinkExt, StreamExt, TryFutureExt};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use snafu::ResultExt;
use event::{Bucket, Event, Metric, MetricValue, Quantile};
use prometheus::{MetricGroup, GroupKind};

use crate::http::{Auth, HttpClient};
use crate::tls::{TlsConfig, TlsOptions, TlsSettings};
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;
use crate::sources::Source;
use crate::config::{
    serialize_duration, deserialize_duration, default_interval, default_false,
    SourceConfig, SourceContext, DataType, ticker_from_duration,
    ProxyConfig, GenerateConfig, SourceDescription
};


// pulled up, and split over multiple lines, because the long lines trip up rustfmt such that it
// gave up trying to format, but reported no error
static PARSE_ERROR_NO_PATH: &str = "No path is set on the endpoint and we got a parse error,\
                                    did you mean to use /metrics? This behavior changed in version 0.11.";
static NOT_FOUND_NO_PATH: &str = "No path is set on the endpoint and we got a 404,\
                                  did you mean to use /metrics?\
                                  This behavior changed in version 0.11.";

#[derive(Debug, Deserialize, Serialize)]
struct PrometheusScrapeConfig {
    endpoints: Vec<String>,
    #[serde(default = "default_interval")]
    #[serde(serialize_with = "serialize_duration", deserialize_with = "deserialize_duration")]
    interval: chrono::Duration,
    #[serde(default = "default_false")]
    honor_labels: bool,
    tls: Option<TlsOptions>,
    auth: Option<Auth>,
}

impl GenerateConfig for PrometheusScrapeConfig {
    fn generate_config() -> Value {
        serde_yaml::to_value(Self {
            endpoints: vec![
                "http://127.0.0.1:1111/metrics".to_string(),
                "http://127.0.0.1:2222/metrics".to_string(),
            ],
            interval: default_interval(),
            honor_labels: false,
            tls: None,
            auth: None,
        }).unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<PrometheusScrapeConfig>("prometheus_scrape")
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_scrape")]
impl SourceConfig for PrometheusScrapeConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let urls = self.endpoints
            .iter()
            .map(|s| s.parse::<http::Uri>().context(crate::sources::UriParseError))
            .collect::<Result<Vec<http::Uri>, crate::sources::BuildError>>()?;
        let tls = TlsSettings::from_options(&self.tls)?;
        Ok(scrape(
            urls,
            tls,
            self.auth.clone(),
            ctx.proxy,
            Some("instance".to_string()),
            self.honor_labels,
            self.interval,
            ctx.shutdown,
            ctx.out,
        ))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "prometheus_scrape"
    }
}

fn scrape(
    urls: Vec<http::Uri>,
    tls: TlsSettings,
    auth: Option<Auth>,
    proxy: ProxyConfig,
    instance_tag: Option<String>,
    honor_labels: bool,
    interval: chrono::Duration,
    shutdown: ShutdownSignal,
    output: Pipeline,
) -> Source {
    let output = output.sink_map_err(|err| {
        error!(
            message = "Error sending metric",
            %err
        );
    });

    let ticker = ticker_from_duration(interval).unwrap();

    Box::pin(
        ticker.take_until(shutdown)
            .map(move |_| futures::stream::iter(urls.clone()))
            .flatten()
            .map(move |url| {
                let instance = instance_tag.as_ref().map(|tag| {
                    let instance = format!(
                        "{}:{}",
                        url.host().unwrap_or_default(),
                        url.port_u16().unwrap_or_else(|| match url.scheme() {
                            Some(scheme) if scheme == &http::uri::Scheme::HTTP => 80,
                            Some(scheme) if scheme == &http::uri::Scheme::HTTPS => 443,
                            _ => 0
                        })
                    );

                    instance
                });

                let client = HttpClient::new(tls.clone(), &proxy)
                    .expect("Building HTTP client failed");
                let mut req = http::Request::get(&url)
                    .body(hyper::body::Body::empty())
                    .expect("error creating request");
                if let Some(auth) = &auth {
                    auth.apply(&mut req);
                }

                client.send(req)
                    .map_err(crate::Error::from)
                    .and_then(|resp| async move {
                        let (header, body) = resp.into_parts();
                        let body = hyper::body::to_bytes(body).await?;
                        Ok((header, body))
                    })
                    .into_stream()
                    .filter_map(move |resp| {
                        let instance = instance.clone();

                        std::future::ready(match resp {
                            Ok((header, body)) if header.status == hyper::StatusCode::OK => {
                                let body = String::from_utf8_lossy(&body);

                                match prometheus::parse_text(&body) {
                                    Ok(groups) => {
                                        // TODO: convert
                                        let events = convert_events(groups);
                                        // Some(events)
                                        Some(futures::stream::iter(events).map(move |mut event| {
                                            let metric = event.as_mut_metric();

                                            if let Some(instance) = &instance {
                                                match (honor_labels, metric.tag_value("instance")) {
                                                    (false, Some(old_instance)) => {
                                                        metric.insert_tag(
                                                            "exported_instance".to_string(),
                                                            old_instance,
                                                        );
                                                        metric.insert_tag(
                                                            "instance".to_string(),
                                                            instance.clone(),
                                                        );
                                                    }
                                                    (true, Some(_)) => {}
                                                    (_, None) => {
                                                        metric.insert_tag(
                                                            "instance".to_string(),
                                                            instance.clone(),
                                                        );
                                                    }
                                                }
                                            }

                                            Ok(event)
                                        }))
                                    }
                                    Err(err) => {
                                        // TODO: handle it
                                        None
                                    }
                                }
                            }

                            Ok((header, _)) => {
                                if header.status == hyper::StatusCode::NOT_FOUND && url.path() == "/" {
                                    warn!(
                                        message = NOT_FOUND_NO_PATH,
                                        endpoint = %url
                                    );
                                }

                                None
                            }

                            Err(err) => {
                                warn!(
                                    message = "HTTP request processing error",
                                    %url,
                                    ?err
                                );

                                None
                            }
                        })
                    })
                    .flatten()
            })
            .flatten()
            .forward(output)
            .inspect(|_| info!("Finished sending"))
    )
}

fn convert_events(groups: Vec<MetricGroup>) -> Vec<Event> {
    let mut events = Vec::with_capacity(groups.len());
    let start = Utc::now();

    for group in groups {
        let name = &group.name;
        match group.metrics {
            GroupKind::Counter(map) => {
                for (key, metric) in map {
                    let counter = Metric::sum(
                        name,
                        "",
                        metric.value,
                    )
                        .with_timestamp(utc_timestamp(key.timestamp, start))
                        .with_tags(key.labels);

                    events.push(counter.into());
                }
            }
            GroupKind::Gauge(metrics) | GroupKind::Untyped(metrics) => {
                for (key, metric) in metrics {
                    let gauge = Metric::gauge(
                        name,
                        "",
                        metric.value,
                    )
                        .with_timestamp(utc_timestamp(key.timestamp, start))
                        .with_tags(key.labels);

                    events.push(gauge.into());
                }
            }
            GroupKind::Summary(metrics) => {
                for (key, metric) in metrics {
                    let m = Metric {
                        name: name.to_string(),
                        description: None,
                        tags: Default::default(),
                        unit: None,
                        timestamp: utc_timestamp(key.timestamp, start),
                        value: MetricValue::Summary {
                            count: metric.count as u64,
                            sum: metric.sum,
                            quantiles: metric.quantiles.iter()
                                .map(|q| Quantile { upper: q.quantile, value: q.value })
                                .collect::<Vec<_>>(),
                        },
                    };

                    events.push(m.into());
                }
            }
            GroupKind::Histogram(metrics) => {
                for (key, metric) in metrics {
                    let m = Metric {
                        name: name.to_string(),
                        description: None,
                        tags: Default::default(),
                        unit: None,
                        timestamp: utc_timestamp(key.timestamp, start),
                        value: MetricValue::Histogram {
                            count: metric.count as u64,
                            sum: metric.sum,
                            buckets: metric.buckets
                                .iter()
                                .map(|b| Bucket { upper: b.bucket, count: b.count })
                                .collect::<Vec<_>>(),
                        },
                    };

                    events.push(m.into());
                }
            }
        }
    }

    events
}

fn utc_timestamp(timestamp: Option<i64>, default: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match timestamp {
        None => Some(default),
        Some(timestamp) => {
            Utc.timestamp_opt(timestamp / 1000, (timestamp % 1000) as u32 * 1000000)
                .latest()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn scrape_honor_labels() {}
}