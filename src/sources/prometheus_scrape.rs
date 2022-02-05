use chrono::{DateTime, TimeZone, Utc};
use event::{Bucket, Event, Metric, Quantile};
use framework::config::{
    default_false, default_interval, deserialize_duration, serialize_duration, DataType,
    GenerateConfig, Output, ProxyConfig, SourceConfig, SourceContext, SourceDescription,
};
use framework::http::{Auth, HttpClient};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::tls::{MaybeTlsSettings, TlsConfig};
use framework::Source;
use futures::{FutureExt, StreamExt, TryFutureExt};
use prometheus::{GroupKind, MetricGroup};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use tokio_stream::wrappers::IntervalStream;

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
    #[serde(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    interval: std::time::Duration,
    #[serde(default = "default_false")]
    honor_labels: bool,
    tls: Option<TlsConfig>,
    auth: Option<Auth>,
}

impl GenerateConfig for PrometheusScrapeConfig {
    fn generate_config() -> String {
        format!(
            r#"
# Endpoints to scrape metrics from.
#
endpoints:
- http://localhost:9090/metrics

# The interval between scrapes.
#
# interval: 15s

# Controls how tag conflicts are handled if the scraped source has tags
# that Vertex would add. If true Vertex will not add the new tag if the
# scraped metric has the tag already. If false, Vertex will rename the
# conflicting tag by adding "exported_" to it. This matches Prometheus's
# "honor_labels" configuration.
#
# honor_labels: false

# Configures the TLS options for outgoing connections.
#
# tls:
{}

# Configures the authentication strategy.
#
# auth:
{}

"#,
            TlsConfig::generate_commented_with_indent(2),
            Auth::generate_commented_with_indent(2)
        )
    }
}

inventory::submit! {
    SourceDescription::new::<PrometheusScrapeConfig>("prometheus_scrape")
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_scrape")]
impl SourceConfig for PrometheusScrapeConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let urls = self
            .endpoints
            .iter()
            .map(|s| {
                s.parse::<http::Uri>()
                    .context(crate::sources::UriParseError)
            })
            .collect::<Result<Vec<http::Uri>, crate::sources::BuildError>>()?;
        let tls = MaybeTlsSettings::from_config(&self.tls, true)?;
        Ok(scrape(
            urls,
            tls,
            self.auth.clone(),
            ctx.proxy,
            Some("instance".to_string()),
            self.honor_labels,
            self.interval,
            ctx.shutdown,
            ctx.output,
        ))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        "prometheus_scrape"
    }
}

fn scrape(
    urls: Vec<http::Uri>,
    tls: MaybeTlsSettings,
    auth: Option<Auth>,
    proxy: ProxyConfig,
    instance_tag: Option<String>,
    honor_labels: bool,
    interval: std::time::Duration,
    shutdown: ShutdownSignal,
    mut output: Pipeline,
) -> Source {
    Box::pin(async move {
        let mut stream = IntervalStream::new(tokio::time::interval(interval))
            .take_until(shutdown)
            .map(move |_| futures::stream::iter(urls.clone()))
            .flatten()
            .map(move |url| {
                let instance = instance_tag.as_ref().map(|_tag| {
                    let instance = format!(
                        "{}:{}",
                        url.host().unwrap_or_default(),
                        url.port_u16().unwrap_or_else(|| match url.scheme() {
                            Some(scheme) if scheme == &http::uri::Scheme::HTTP => 80,
                            Some(scheme) if scheme == &http::uri::Scheme::HTTPS => 443,
                            _ => 0,
                        })
                    );

                    instance
                });

                let client =
                    HttpClient::new(tls.clone(), &proxy).expect("Building HTTP client failed");
                let mut req = http::Request::get(&url)
                    .body(hyper::body::Body::empty())
                    .expect("error creating request");
                if let Some(auth) = &auth {
                    auth.apply(&mut req);
                }

                client
                    .send(req)
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
                                                            "exported_instance",
                                                            old_instance,
                                                        );
                                                        metric.insert_tag(
                                                            "instance",
                                                            instance.clone(),
                                                        );
                                                    }
                                                    (true, Some(_)) => {}
                                                    (_, None) => {
                                                        metric.insert_tag(
                                                            "instance",
                                                            instance.clone(),
                                                        );
                                                    }
                                                }
                                            }

                                            event
                                        }))
                                    }
                                    Err(_err) => {
                                        // TODO: handle it
                                        None
                                    }
                                }
                            }

                            Ok((header, _)) => {
                                if header.status == hyper::StatusCode::NOT_FOUND
                                    && url.path() == "/"
                                {
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
            .boxed();

        match output.send_all(&mut stream).await {
            Ok(()) => {
                info!(message = "Finished sending");
                Ok(())
            }
            Err(err) => {
                error!(
                    message = "Error sending scraped metrics",
                    %err
                );

                Err(())
            }
        }
    })
}

fn convert_events(groups: Vec<MetricGroup>) -> Vec<Event> {
    let mut events = Vec::with_capacity(groups.len());
    let start = Utc::now();

    for group in groups {
        let name = &group.name;
        match group.metrics {
            GroupKind::Counter(map) => {
                for (key, metric) in map {
                    let counter = Metric::sum(name, "", metric.value)
                        .with_timestamp(utc_timestamp(key.timestamp, start))
                        .with_tags(key.labels);

                    events.push(counter.into());
                }
            }
            GroupKind::Gauge(metrics) | GroupKind::Untyped(metrics) => {
                for (key, metric) in metrics {
                    let gauge = Metric::gauge(name, "", metric.value)
                        .with_timestamp(utc_timestamp(key.timestamp, start))
                        .with_tags(key.labels);

                    events.push(gauge.into());
                }
            }
            GroupKind::Summary(metrics) => {
                for (key, metric) in metrics {
                    let m = Metric::summary(
                        name,
                        "",
                        metric.count,
                        metric.sum,
                        metric
                            .quantiles
                            .iter()
                            .map(|q| Quantile {
                                quantile: q.quantile,
                                value: q.value,
                            })
                            .collect::<Vec<_>>(),
                    )
                    .with_timestamp(utc_timestamp(key.timestamp, start));

                    events.push(m.into());
                }
            }
            GroupKind::Histogram(metrics) => {
                for (key, metric) in metrics {
                    let m = Metric::histogram(
                        name,
                        "",
                        metric.count,
                        metric.sum,
                        metric
                            .buckets
                            .iter()
                            .map(|b| Bucket {
                                upper: b.bucket,
                                count: b.count,
                            })
                            .collect::<Vec<_>>(),
                    )
                    .with_timestamp(utc_timestamp(key.timestamp, start));

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
        Some(timestamp) => Utc
            .timestamp_opt(timestamp / 1000, (timestamp % 1000) as u32 * 1000000)
            .latest(),
    }
}

#[cfg(test)]
mod tests {
    use crate::sources::prometheus_scrape::PrometheusScrapeConfig;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<PrometheusScrapeConfig>();
    }

    #[tokio::test]
    #[ignore]
    async fn scrape_honor_labels() {
        todo!()
    }
}
