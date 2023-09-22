use std::task::{Context, Poll};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use configurable::configurable_component;
use event::{Event, Metric};
use framework::batch::{BatchConfig, EncodedEvent, SinkBatchSettings};
use framework::config::{DataType, SinkConfig, SinkContext};
use framework::http::{Auth, HttpClient};
use framework::sink::util::http::HttpRetryLogic;
use framework::sink::util::service::RequestConfig;
use framework::sink::util::sink::PartitionBatchSink;
use framework::sink::util::{
    MetricNormalize, MetricNormalizer, MetricSet, MetricsBuffer, PartitionBuffer,
    PartitionInnerBuffer,
};
use framework::template::Template;
use framework::tls::TlsConfig;
use framework::{Healthcheck, HealthcheckError, Sink};
use futures::{future::BoxFuture, stream, FutureExt, SinkExt};
use http::{StatusCode, Uri};
use hyper::{body, Body};
use prost::Message;
use tower::{Service, ServiceBuilder};

use crate::sinks::BuildError;

#[derive(Copy, Clone, Debug, Default)]
pub struct PrometheusRemoteWriteDefaultBatchSettings;

impl SinkBatchSettings for PrometheusRemoteWriteDefaultBatchSettings {
    const MAX_EVENTS: Option<usize> = Some(1000);
    const MAX_BYTES: Option<usize> = None;
    const TIMEOUT: Duration = Duration::from_secs(1);
}

#[configurable_component(sink, name = "prometheus_remote_write")]
#[serde(deny_unknown_fields)]
pub struct RemoteWriteConfig {
    /// Endpoint of Prometheus's remote write API.
    #[configurable(required, format = "uri", example = "http://10.1.1.1:8000")]
    pub endpoint: String,

    #[serde(default)]
    pub batch: BatchConfig<PrometheusRemoteWriteDefaultBatchSettings>,

    /// Configures the sink request behavior.
    #[serde(default)]
    pub request: RequestConfig,

    /// Tenant ID is a special label for Cortex, Thanos or VictoriaMetrics.
    #[serde(default)]
    pub tenant_id: Option<Template>,

    pub tls: Option<TlsConfig>,
    pub auth: Option<Auth>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_remote_write")]
impl SinkConfig for RemoteWriteConfig {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let endpoint = self.endpoint.parse::<Uri>().map_err(BuildError::UriParse)?;
        let batch = self.batch.into_batch_settings()?;
        let request = self.request.unwrap_with(&RequestConfig::default());

        let client = HttpClient::new(&self.tls, cx.proxy())?;
        let tenant_id = self.tenant_id.clone();
        let auth = self.auth.clone();

        let healthcheck = healthcheck(endpoint.clone(), client.clone()).boxed();
        let service = RemoteWriteService {
            endpoint,
            client,
            auth,
        };

        let sink = {
            let service = request.service(HttpRetryLogic, service);
            let service = ServiceBuilder::new().service(service);
            let buffer = PartitionBuffer::new(MetricsBuffer::new(batch.size));
            let mut normalizer = MetricNormalizer::<PrometheusMetricNormalize>::default();

            PartitionBatchSink::new(service, buffer, batch.timeout)
                .with_flat_map(move |event: Event| {
                    let byte_size = event.size_of();
                    stream::iter(normalizer.apply(event.into_metric()).map(|event| {
                        let tenant_id = tenant_id.as_ref().and_then(|template| {
                            template
                                .render_string(&event)
                                .map_err(|err| {
                                    error!(message = "Failed to render template", ?err);

                                    // TODO: metrics
                                    // emit!(&TemplateRenderingFailed {
                                    //     err,
                                    //     field: Some("tenant_id"),
                                    //     drop_event: false,
                                    // })
                                })
                                .ok()
                        });

                        let key = PartitionKey { tenant: tenant_id };
                        Ok(EncodedEvent::new(
                            PartitionInnerBuffer::new(event, key),
                            byte_size,
                        ))
                    }))
                })
                .sink_map_err(|err| error!(message = "Prometheus remote write sink error", %err))
        };

        Ok((Sink::from_event_sink(sink), healthcheck))
    }

    fn input_type(&self) -> DataType {
        DataType::Metric
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct PartitionKey {
    tenant: Option<String>,
}

async fn healthcheck(endpoint: Uri, client: HttpClient) -> crate::Result<()> {
    let req = http::Request::get(endpoint).body(Body::empty())?;

    let resp = client.send(req).await?;

    match resp.status() {
        StatusCode::OK => Ok(()),
        other => Err(HealthcheckError::UnexpectedStatus(other).into()),
    }
}

#[derive(Clone)]
struct RemoteWriteService {
    endpoint: Uri,
    client: HttpClient,
    auth: Option<Auth>,
}

impl RemoteWriteService {
    fn encode_events(&self, metrics: Vec<Metric>) -> Bytes {
        let mut timeseries = crate::common::prometheus::TimeSeries::new();
        for metric in metrics {
            timeseries.encode_metric(&metric);
        }

        let wr = timeseries.finish();
        let mut out = BytesMut::with_capacity(wr.encoded_len());
        wr.encode(&mut out).expect("Out of memory");
        out.freeze()
    }
}

impl Service<PartitionInnerBuffer<Vec<Metric>, PartitionKey>> for RemoteWriteService {
    type Response = http::Response<Bytes>;
    type Error = crate::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, buf: PartitionInnerBuffer<Vec<Metric>, PartitionKey>) -> Self::Future {
        let (events, key) = buf.into_parts();
        let body = self.encode_events(events);
        let body = snap_block(body);

        let mut builder = http::Request::post(&self.endpoint)
            .header("X-Prometheus-Remote-Write-Version", "0.1.0")
            .header("Content-Encoding", "snappy")
            .header("Content-Type", "application/x-protobuf");
        if let Some(tenant_id) = key.tenant {
            builder = builder.header("X-Scope-OrgID", tenant_id);
        }

        let mut req = builder.body(body.into()).unwrap();
        if let Some(auth) = &self.auth {
            auth.apply(&mut req);
        }

        let client = self.client.clone();

        Box::pin(async move {
            let resp = client.send(req).await?;
            let (parts, body) = resp.into_parts();
            let body = body::to_bytes(body).await?;
            Ok(hyper::Response::from_parts(parts, body))
        })
    }
}

fn snap_block(data: Bytes) -> Vec<u8> {
    snap::raw::Encoder::new()
        .compress_vec(&data)
        .expect("Out of memory")
}

#[derive(Default)]
pub struct PrometheusMetricNormalize;

impl MetricNormalize for PrometheusMetricNormalize {
    fn apply_state(&mut self, _state: &mut MetricSet, metric: Metric) -> Option<Metric> {
        Some(metric)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use event::{btreemap, Metric};
    use framework::sink::util::testing::build_test_server;
    use futures_util::StreamExt;
    use http::HeaderMap;
    use prometheus::proto;
    use std::collections::BTreeMap;
    use testify::next_addr;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<RemoteWriteConfig>();
    }

    macro_rules! labels {
        ( $( $name:expr => $value:expr ),* ) => {
            vec![ $( proto::Label {
                name: $name.to_string(),
                value: $value.to_string()
            }, )* ]
        };
    }

    fn test_gauge(name: impl Into<String>, value: f64, tags: BTreeMap<String, String>) -> Event {
        Metric::gauge_with_tags(name, "", value, tags)
            .with_timestamp(Some(Utc::now()))
            .into()
    }

    async fn send_request(
        config: &str,
        events: Vec<Event>,
    ) -> Vec<(HeaderMap, proto::WriteRequest)> {
        let addr = next_addr();
        let (rx, trigger, server) = build_test_server(addr);
        tokio::spawn(server);

        let config = format!("endpoint: \"http://{}/write\"\n{}", addr, config);
        let config: RemoteWriteConfig = serde_yaml::from_str(&config).unwrap();
        let cx = SinkContext::new_test();

        let (sink, _healthcheck) = config.build(cx).await.unwrap();
        sink.run_events(events).await.unwrap();

        drop(trigger);

        rx.map(|(parts, body)| {
            assert_eq!(parts.method, "POST");
            assert_eq!(parts.uri.path(), "/write");
            let headers = parts.headers;
            assert_eq!(headers["x-prometheus-remote-write-version"], "0.1.0");
            assert_eq!(headers["content-encoding"], "snappy");
            assert_eq!(headers["content-type"], "application/x-protobuf");

            if config.auth.is_some() {
                assert!(headers.contains_key("authorization"));
            }

            let decoded = snap::raw::Decoder::new()
                .decompress_vec(&body)
                .expect("Invalid snappy compressed data");

            let request =
                proto::WriteRequest::decode(Bytes::from(decoded)).expect("Invalid protobuf");

            (headers, request)
        })
        .collect::<Vec<_>>()
        .await
    }

    #[tokio::test]
    async fn sends_request() {
        let outputs = send_request(
            "",
            vec![test_gauge(
                "gauge_2",
                32.0,
                btreemap!("foo" => "bar", "bar" => "foo"),
            )],
        )
        .await;
        assert_eq!(outputs.len(), 1);
        let (headers, req) = &outputs[0];

        assert!(!headers.contains_key("x-scope-orgid"));

        assert_eq!(req.timeseries.len(), 1);
        assert_eq!(
            req.timeseries[0].labels,
            labels!(prometheus::METRIC_NAME_LABEL => "gauge_2", "bar" => "foo", "foo" => "bar")
        );
        assert_eq!(req.timeseries[0].samples.len(), 1);
        assert_eq!(req.timeseries[0].samples[0].value, 32.0);
        assert_eq!(req.metadata.len(), 1);
        assert_eq!(req.metadata[0].r#type, proto::MetricType::Gauge as i32);
        assert_eq!(req.metadata[0].metric_family_name, "gauge_2");
    }

    #[tokio::test]
    async fn sends_with_authenticated() {
        let outputs = send_request(
            "auth:\n  strategy: basic\n  user: user\n  password: password",
            vec![test_gauge("gauge_2", 32.0, btreemap!("foo" => "bar"))],
        )
        .await;

        assert_eq!(outputs.len(), 1);
        let (_headers, req) = &outputs[0];

        assert_eq!(req.timeseries.len(), 1);
        assert_eq!(
            req.timeseries[0].labels,
            labels!(prometheus::METRIC_NAME_LABEL => "gauge_2", "foo" => "bar")
        );
        assert_eq!(req.timeseries[0].samples.len(), 1);
        assert_eq!(req.timeseries[0].samples[0].value, 32.0);
        assert_eq!(req.metadata.len(), 1);
        assert_eq!(req.metadata[0].r#type, proto::MetricType::Gauge as i32);
        assert_eq!(req.metadata[0].metric_family_name, "gauge_2");
    }

    #[tokio::test]
    async fn send_x_scope_orgid_header() {
        let outputs = send_request(
            "tenant_id: tenant",
            vec![test_gauge("gauge_3", 12.1, btreemap!())],
        )
        .await;

        assert_eq!(outputs.len(), 1);
        let (headers, _) = &outputs[0];
        assert_eq!(headers["x-scope-orgid"], "tenant");
    }

    #[tokio::test]
    async fn sends_templated_x_scope_orgid_header() {
        let outputs = send_request(
            "tenant_id: tenant_%Y",
            vec![test_gauge("gauge_3", 12.3, btreemap!())],
        )
        .await;

        assert_eq!(outputs.len(), 1);
        let (headers, _) = &outputs[0];
        let orgid = headers["x-scope-orgid"]
            .to_str()
            .expect("Missing x-scope-orgid header");

        assert!(orgid.starts_with("tenant_20"));
        assert_eq!(orgid.len(), 11);
    }
}

#[cfg(all(test, feature = "integration-tests-prometheus_remote_write"))]
mod integration_tests {
    use std::time::Duration;

    use super::RemoteWriteConfig;
    use crate::testing::{ContainerBuilder, WaitFor};
    use event::Metric;
    use framework::config::{ProxyConfig, SinkConfig, SinkContext};
    use framework::http::HttpClient;
    use hyper::Body;
    use serde::{Deserialize, Serialize};

    #[tokio::test]
    async fn cortex_write_and_query() {
        // 1. Setup Cortex
        let container = ContainerBuilder::new("ubuntu/cortex:latest")
            .with_env("TZ", "UTC")
            .port(9009)
            .run()
            .unwrap();
        container.wait(WaitFor::Stderr("Cortex started")).unwrap();
        let address = container.get_host_port(9009).unwrap();

        // 2. Setup sink
        let config = format!("endpoint: http://{}/api/v1/push", address);
        let config: RemoteWriteConfig = serde_yaml::from_str(&config).unwrap();
        let cx = SinkContext::new_test();

        let (sink, _healthcheck) = config.build(cx).await.unwrap();
        sink.run_events(vec![Metric::gauge("foo", "", 1.1).into()])
            .await
            .unwrap();

        // wait until all events flushed
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 3. Query label values
        let endpoint = format!("http://{}/prometheus/api/v1/label/__name__/values", address);
        let client = HttpClient::new(&None, &ProxyConfig::default()).unwrap();

        let req = http::Request::get(endpoint).body(Body::empty()).unwrap();

        let resp = client.send(req).await.unwrap();

        // 4. Assert response
        let (parts, body) = resp.into_parts();
        assert!(parts.status.is_success());

        #[derive(Debug, Deserialize, Serialize)]
        struct QueryResp {
            status: String,
            data: Vec<String>,
        }

        let body = hyper::body::to_bytes(body).await.unwrap();
        let qr: QueryResp = serde_json::from_slice(body.as_ref()).unwrap();
        assert_eq!(qr.status, "success".to_string());
        assert_eq!(qr.data.len(), 1);
        assert_eq!(qr.data[0], "foo");
    }
}
