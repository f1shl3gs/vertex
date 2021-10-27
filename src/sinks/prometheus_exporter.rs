use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use std::{
    net::{SocketAddr, IpAddr, Ipv4Addr},
    sync::{Arc, RwLock},
};
use std::fmt::Write;
use crate::{
    sinks::{
        Sink,
        StreamSink,
    },
    config::{SinkConfig, SinkContext, DataType, Resource, HealthCheck},
    tls::TLSConfig,
};
use event::{Event, Metric};
use futures::prelude::stream::BoxStream;
use stream_cancel::{Trigger, Tripwire};
use crate::buffers::Acker;
use chrono::Utc;
use std::hash::Hasher;
use indexmap::set::IndexSet;
use std::ops::{Deref, DerefMut};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use hyper::{Request, Body, Method, Response, StatusCode, Server};
use futures::{StreamExt, FutureExt};
use event::MetricValue;
use hyper::http::HeaderValue;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusExporterConfig {
    pub namespace: Option<String>,

    pub tls: Option<TLSConfig>,

    #[serde(default = "default_listen_address")]
    pub listen: SocketAddr,

    #[serde(default = "default_telemetry_path")]
    pub telemetry_path: String,

    pub compression: Option<bool>,
}

impl Default for PrometheusExporterConfig {
    fn default() -> Self {
        Self {
            namespace: None,
            tls: None,
            listen: default_listen_address(),
            telemetry_path: default_telemetry_path(),
            compression: None,
        }
    }
}

fn default_listen_address() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9100)
}

fn default_telemetry_path() -> String {
    "/metrics".into()
}

#[async_trait]
#[typetag::serde(name = "prometheus_exporter")]
impl SinkConfig for PrometheusExporterConfig {
    async fn build(&self, ctx: SinkContext) -> crate::Result<(Sink, HealthCheck)> {
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
        let metric = &self.metric;
        metric.tags.hash(state);
        metric.name.hash(state);

        // TODO: maybe handle metric value too?
    }
}

impl PartialEq<Self> for ExpiringEntry {
    fn eq(&self, other: &Self) -> bool {
        self.metric.tags == other.metric.tags && self.metric.name == other.metric.name
    }
}

impl std::cmp::Eq for ExpiringEntry {}

struct PrometheusExporter {
    acker: Acker,
    shutdown_trigger: Option<Trigger>,
    config: PrometheusExporterConfig,
    metrics: Arc<RwLock<IndexSet<ExpiringEntry>>>,
}

fn handle(
    req: Request<Body>,
    metrics: &IndexSet<ExpiringEntry>,
    now: i64,
) -> Response<Body> {
    let mut resp = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            let s = metrics
                .iter()
                .filter(|ent| ent.expired_at > now)
                .fold(String::new(), |mut result, ent| {
                    match ent.metric.value {
                        MetricValue::Gauge(v) | MetricValue::Sum(v) => {
                            if ent.tags.is_empty() {
                                writeln!(
                                    &mut result,
                                    "{} {}",
                                    ent.name,
                                    v
                                ).unwrap();
                            } else {
                                writeln!(
                                    &mut result,
                                    "{}{{{}}} {}",
                                    ent.name,
                                    ent.tags
                                        .iter()
                                        .map(|(k, v)| format!("{}=\"{}\"", k, v))
                                        .collect::<Vec<String>>()
                                        .join(","),
                                    v
                                ).unwrap();
                            }
                        }
                        _ => unreachable!()
                    }

                    result
                });

            resp.headers_mut().insert("Content-Type", HeaderValue::from_static("text/plain; charset=utf-8"));

            *resp.body_mut() = Body::from(s);
        }

        _ => {
            *resp.status_mut() = StatusCode::NOT_FOUND;
        }
    }

    resp
}

async fn tripwire_handler(closed: bool) {
    futures::future::poll_fn(|_| {
        if closed {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    })
        .await
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
        tokio::spawn(async move {
/*
            let tls = MaybeTLSSettings::from_config(&self.config.tls)
                .map_err(|err| warn!(message = "Server TLS error: {}", err))?;

            let listener = tls.bind(&address)
                .await
                .map_err(|err| warn!(message = "Server bind error: {}", err))?;
*/
            /*Server::builder()
                .serve(new_service)
                .with_graceful_shutdown(tripwire.then(tripwire_handler))
                .await
                .map_err(|err| warn!(message = "Server error", ?err))?;
*/
            Server::bind(&address)
                .serve(new_service)
                .with_graceful_shutdown(tripwire.then(tripwire_handler))
                .await
                .map_err(|err| eprintln!("Server error: {}", err))?;

            Ok::<(), ()>(())
        });

        self.shutdown_trigger = Some(trigger);
    }
}

#[async_trait]
impl StreamSink for PrometheusExporter {
    async fn run(&mut self, mut input: BoxStream<'_, Event>) -> Result<(), ()> {
        self.start_server_if_needed().await;

        let expiration = 5 * 60;

        while let Some(event) = input.next().await {
            let metric = event.into_metric();

            let mut metrics = self.metrics.write().unwrap();
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

            metrics.replace(entry);
            self.acker.ack(1);
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
        let m1 = Metric {
            name: "foo".into(),
            description: None,
            tags: Default::default(),
            unit: None,
            timestamp: None,
            value: MetricValue::Gauge(0.1),
        };
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
        assert_eq!(set.iter().enumerate().nth(0).unwrap().1.expired_at, now + 60);
    }
}