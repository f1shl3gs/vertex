use std::convert::Infallible;
use std::fmt::Write;
use std::hash::Hasher;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use buffers::Acker;
use chrono::Utc;
use event::Metric;
use event::{Events, MetricValue};
use framework::config::{
    default_false, DataType, GenerateConfig, Resource, SinkConfig, SinkContext, SinkDescription,
};
use framework::stream::tripwire_handler;
use framework::tls::{MaybeTlsSettings, TlsConfig};
use framework::{Healthcheck, Sink, StreamSink};
use futures::prelude::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use hyper::http::HeaderValue;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use indexmap::set::IndexSet;
use serde::{Deserialize, Serialize};
use stream_cancel::{Trigger, Tripwire};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusExporterConfig {
    pub tls: Option<TlsConfig>,

    #[serde(default = "default_listen_address")]
    pub listen: SocketAddr,

    #[serde(default = "default_telemetry_path")]
    pub telemetry_path: String,

    #[serde(default = "default_false")]
    pub compression: bool,
}

impl Default for PrometheusExporterConfig {
    fn default() -> Self {
        Self {
            tls: None,
            listen: default_listen_address(),
            telemetry_path: default_telemetry_path(),
            compression: default_false(),
        }
    }
}

fn default_listen_address() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9100)
}

fn default_telemetry_path() -> String {
    "/metrics".into()
}

impl GenerateConfig for PrometheusExporterConfig {
    fn generate_config() -> String {
        r#"
# Which address the prometheus server will listen at
# lisent: 0.0.0.0:9100

# Telemetry path for this HTTP server
# telemetry_path: /metric

# Compress response
# compression: false

"#
        .into()
    }
}

inventory::submit! {
    SinkDescription::new::<PrometheusExporterConfig>("prometheus_exporter")
}

#[async_trait]
#[typetag::serde(name = "prometheus_exporter")]
impl SinkConfig for PrometheusExporterConfig {
    async fn build(&self, ctx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let sink = PrometheusExporter::new(self.clone(), ctx.acker);
        let health_check = futures::future::ok(()).boxed();

        Ok((Sink::Stream(Box::new(sink)), health_check))
    }

    fn input_type(&self) -> DataType {
        DataType::Metric
    }

    fn sink_type(&self) -> &'static str {
        "prometheus_exporter"
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.listen)]
    }
}

struct ExpiringEntry {
    metric: Metric,
    expired_at: i64,
}

impl Deref for ExpiringEntry {
    type Target = Metric;

    fn deref(&self) -> &Self::Target {
        &self.metric
    }
}

impl DerefMut for ExpiringEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.metric
    }
}

impl std::hash::Hash for ExpiringEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.series.hash(state)
    }
}

impl PartialEq<Self> for ExpiringEntry {
    fn eq(&self, other: &Self) -> bool {
        self.series.eq(&other.series)
    }
}

impl std::cmp::Eq for ExpiringEntry {}

struct PrometheusExporter {
    acker: Acker,
    shutdown_trigger: Option<Trigger>,
    config: PrometheusExporterConfig,
    metrics: Arc<RwLock<IndexSet<ExpiringEntry>>>,
}

macro_rules! write_metric {
    ($dst:expr, $name:expr, $tags:expr, $value:expr) => {
        if $tags.is_empty() {
            writeln!(&mut $dst, "{} {}", $name.to_owned(), $value).unwrap();
        } else {
            writeln!(
                &mut $dst,
                "{}{{{}}} {}",
                $name,
                $tags
                    .iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect::<Vec<String>>()
                    .join(","),
                $value
            )
            .unwrap();
        }
    };
}

fn handle(req: Request<Body>, metrics: &IndexSet<ExpiringEntry>, now: i64) -> Response<Body> {
    let mut resp = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            let s = metrics.iter().filter(|ent| ent.expired_at > now).fold(
                String::new(),
                |mut result, ent| {
                    match ent.metric.value {
                        MetricValue::Gauge(v) | MetricValue::Sum(v) => {
                            write_metric!(result, ent.name(), ent.tags(), v);
                        }
                        MetricValue::Summary {
                            ref quantiles,
                            sum,
                            count,
                        } => {
                            for q in quantiles {
                                let mut tags = ent.tags().clone();
                                tags.insert("quantile".to_string(), q.quantile.to_string());
                                write_metric!(result, ent.name(), tags, q.value)
                            }

                            write_metric!(result, format!("{}_sum", ent.name()), ent.tags(), sum);
                            write_metric!(
                                result,
                                format!("{}_count", ent.name()),
                                ent.tags(),
                                count
                            );
                        }
                        MetricValue::Histogram {
                            ref buckets,
                            sum,
                            count,
                        } => {
                            for b in buckets {
                                let mut tags = ent.tags().clone();
                                tags.insert("le".to_string(), b.upper.to_string());
                                write_metric!(result, ent.name(), tags, b.count);
                            }

                            write_metric!(result, format!("{}_sum", ent.name()), ent.tags(), sum);
                            write_metric!(
                                result,
                                format!("{}_count", ent.name()),
                                ent.tags(),
                                count
                            );
                        }
                    }

                    result
                },
            );

            resp.headers_mut().insert(
                "Content-Type",
                HeaderValue::from_static("text/plain; charset=utf-8"),
            );

            *resp.body_mut() = Body::from(s);
        }

        _ => {
            *resp.status_mut() = StatusCode::NOT_FOUND;
        }
    }

    resp
}

impl PrometheusExporter {
    fn new(config: PrometheusExporterConfig, acker: Acker) -> Self {
        Self {
            acker,
            config,
            shutdown_trigger: None,
            metrics: Arc::new(RwLock::new(IndexSet::<ExpiringEntry>::new())),
        }
    }

    async fn start_server_if_needed(&mut self) {
        if self.shutdown_trigger.is_some() {
            return;
        }

        let metrics = Arc::clone(&self.metrics);
        // let namespace = self.config.namespace.clone();

        let new_service = make_service_fn(move |_| {
            let metrics = Arc::clone(&metrics);

            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let metrics = metrics.read().unwrap();
                    let now = Utc::now().timestamp();

                    let resp = handle(req, &metrics, now);

                    futures::future::ok::<_, Infallible>(resp)
                }))
            }
        });

        let (trigger, tripwire) = Tripwire::new();
        let address = self.config.listen;
        let tls = self.config.tls.clone();
        tokio::spawn(async move {
            let tls = MaybeTlsSettings::from_config(&tls, true).map_err(|err| {
                error!(message = "Server TLS error", ?err);
            })?;
            let listener = tls.bind(&address).await.map_err(|err| {
                error!(message = "Server TLS error", ?err);
            })?;

            Server::builder(hyper::server::accept::from_stream(listener.accept_stream()))
                .serve(new_service)
                .with_graceful_shutdown(tripwire.then(tripwire_handler))
                .await
                .map_err(|err| {
                    error!(
                        message = "Server error",
                        %err
                    );
                })?;

            Ok::<(), ()>(())
        });

        self.shutdown_trigger = Some(trigger);
    }
}

#[async_trait]
impl StreamSink for PrometheusExporter {
    async fn run(mut self: Box<Self>, mut input: BoxStream<'_, Events>) -> Result<(), ()> {
        self.start_server_if_needed().await;

        let expiration = 5 * 60;

        while let Some(events) = input.next().await {
            if let Events::Metrics(metrics) = events {
                let mut state = self.metrics.write().unwrap();

                metrics.into_iter().for_each(|metric| {
                    let entry = match metric.timestamp {
                        None => {
                            let now = Utc::now().timestamp();
                            ExpiringEntry {
                                metric,
                                expired_at: now + expiration,
                            }
                        }
                        Some(timestamp) => {
                            let ts = timestamp.timestamp();

                            ExpiringEntry {
                                metric,
                                expired_at: ts + expiration,
                            }
                        }
                    };

                    state.replace(entry);
                    self.acker.ack(1);
                })
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_insert() {
        let mut set = IndexSet::new();
        let m1 = Metric::gauge("foo", "", 0.1);
        let mut m2 = m1.clone();
        m2.value = MetricValue::Gauge(0.2);

        let now = Utc::now().timestamp();
        let ent = ExpiringEntry {
            metric: m1,
            expired_at: now + 60,
        };

        set.insert(ent);

        assert_eq!(set.len(), 1);

        let ent = ExpiringEntry {
            metric: m2,
            expired_at: now + 120,
        };

        set.insert(ent);

        assert_eq!(set.len(), 1);
        assert_eq!(
            set.iter().enumerate().next().unwrap().1.expired_at,
            now + 60
        );
    }
}
