use std::task::{Context, Poll};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use configurable::configurable_component;
use event::{Event, Metric};
use framework::batch::{BatchConfig, EncodedEvent, SinkBatchSettings};
use framework::config::{InputType, SinkConfig, SinkContext, serde_uri};
use framework::http::{Auth, HttpClient};
use framework::sink::batch::PartitionBatchSink;
use framework::sink::buffer::partition::{PartitionBuffer, PartitionInnerBuffer};
use framework::sink::http::HttpRetryLogic;
use framework::sink::metrics::{MetricNormalize, MetricNormalizer, MetricSet, MetricsBuffer};
use framework::sink::service::RequestConfig;
use framework::template::Template;
use framework::tls::TlsConfig;
use framework::{Healthcheck, HealthcheckError, Sink};
use futures::{FutureExt, SinkExt, future::BoxFuture, stream};
use http::header::{CONTENT_ENCODING, CONTENT_TYPE};
use http::{StatusCode, Uri};
use http_body_util::{BodyExt, Full};
use prost::Message;
use tower::{Service, ServiceBuilder};

#[derive(Copy, Clone, Debug, Default)]
struct PrometheusRemoteWriteDefaultBatchSettings;

impl SinkBatchSettings for PrometheusRemoteWriteDefaultBatchSettings {
    const MAX_EVENTS: Option<usize> = Some(1000);
    const MAX_BYTES: Option<usize> = None;
    const TIMEOUT: Duration = Duration::from_secs(1);
}

#[configurable_component(sink, name = "prometheus_remote_write")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Endpoint of Prometheus's remote write API.
    #[configurable(format = "uri", example = "http://10.1.1.1:8000")]
    #[serde(with = "serde_uri")]
    endpoint: Uri,

    auth: Option<Auth>,

    tls: Option<TlsConfig>,

    /// Configures the sink request behavior.
    #[serde(default)]
    request: RequestConfig,

    #[serde(default)]
    batch: BatchConfig<PrometheusRemoteWriteDefaultBatchSettings>,

    /// Tenant ID is a special label for Cortex, Thanos or VictoriaMetrics.
    #[serde(default)]
    tenant_id: Option<Template>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_remote_write")]
impl SinkConfig for Config {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let batch = self.batch.into_batch_settings()?;
        let request = self.request.settings();

        let client = HttpClient::new(self.tls.as_ref(), cx.proxy())?;
        let tenant_id = self.tenant_id.clone();
        let auth = self.auth.clone();

        let healthcheck = healthcheck(self.endpoint.clone(), client.clone()).boxed();
        let service = RemoteWriteService {
            endpoint: self.endpoint.clone(),
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
                                    error!(message = "Failed to render template", %err);

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

    fn input_type(&self) -> InputType {
        InputType::metric()
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct PartitionKey {
    tenant: Option<String>,
}

async fn healthcheck(endpoint: Uri, client: HttpClient) -> crate::Result<()> {
    let req = http::Request::get(endpoint).body(Full::<Bytes>::default())?;

    let resp = client.send(req).await?;
    let (parts, incoming) = resp.into_parts();

    if parts.status != StatusCode::OK {
        let data = incoming.collect().await?.to_bytes();

        return Err(HealthcheckError::UnexpectedStatus(
            parts.status,
            String::from_utf8_lossy(&data).to_string(),
        )
        .into());
    }

    Ok(())
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
            .header(CONTENT_ENCODING, "snappy")
            .header(CONTENT_TYPE, "application/x-protobuf");
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
            let (parts, incoming) = resp.into_parts();
            let data = incoming.collect().await?.to_bytes();
            Ok(hyper::Response::from_parts(parts, data))
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
    use std::borrow::Cow;

    use chrono::Utc;
    use event::tags::Tags;
    use event::{Metric, tags};
    use framework::sink::testing::build_test_server;
    use http::HeaderMap;
    use prometheus::proto;
    use testify::next_addr;

    use super::*;
    use crate::testing::trace_init;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    macro_rules! labels {
        ( $( $name:expr => $value:expr ),* ) => {
            vec![ $( proto::Label {
                name: $name.to_string(),
                value: $value.to_string()
            }, )* ]
        };
    }

    fn gauge(name: impl Into<Cow<'static, str>>, value: f64, tags: Tags) -> Event {
        Metric::gauge_with_tags(name, "", value, tags)
            .with_timestamp(Some(Utc::now()))
            .into()
    }

    async fn send_request(
        config: &str,
        events: Vec<Event>,
    ) -> Vec<(HeaderMap, proto::WriteRequest)> {
        let addr = next_addr();
        let (mut rx, trigger, server) = build_test_server(addr);
        tokio::spawn(server);

        let config = format!("endpoint: \"http://{addr}/write\"\n{config}");
        let config: Config = serde_yaml::from_str(&config).unwrap();
        let cx = SinkContext::new_test();

        let (sink, _healthcheck) = config.build(cx).await.unwrap();
        sink.run_events(events).await.unwrap();

        drop(trigger);

        let mut requests = Vec::new();
        while let Some((parts, body)) = rx.recv().await {
            assert_eq!(parts.method, "POST");
            assert_eq!(parts.uri.path(), "/write");
            let headers = parts.headers;
            assert_eq!(headers["x-prometheus-remote-write-version"], "0.1.0");
            assert_eq!(headers[CONTENT_ENCODING], "snappy");
            assert_eq!(headers[CONTENT_TYPE], "application/x-protobuf");

            if config.auth.is_some() {
                assert!(headers.contains_key("authorization"));
            }

            let decoded = snap::raw::Decoder::new()
                .decompress_vec(&body)
                .expect("Invalid snappy compressed data");

            let req = proto::WriteRequest::decode(Bytes::from(decoded)).expect("Invalid protobuf");

            requests.push((headers, req));
        }

        requests
    }

    #[tokio::test]
    async fn sends_request() {
        let outputs = send_request(
            "",
            vec![gauge(
                "gauge_2",
                32.0,
                tags!("foo" => "bar", "bar" => "foo"),
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
        trace_init();

        let outputs = send_request(
            "auth:\n  strategy: basic\n  user: user\n  password: password",
            vec![gauge("gauge_2", 32.0, tags!("foo" => "bar"))],
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
        let outputs =
            send_request("tenant_id: tenant", vec![gauge("gauge_3", 12.1, tags!())]).await;

        assert_eq!(outputs.len(), 1);
        let (headers, _) = &outputs[0];
        assert_eq!(headers["x-scope-orgid"], "tenant");
    }

    #[tokio::test]
    async fn sends_templated_x_scope_orgid_header() {
        let outputs = send_request(
            "tenant_id: tenant_%Y",
            vec![gauge("gauge_3", 12.3, tags!())],
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

#[cfg(all(test, feature = "prometheus_remote_write_integration-tests"))]
mod integration_tests {
    use std::time::Duration;

    use super::Config;
    use crate::testing::trace_init;
    use bytes::Bytes;
    use event::Metric;
    use framework::config::{ProxyConfig, SinkConfig, SinkContext};
    use framework::http::HttpClient;
    use http_body_util::{BodyExt, Full};
    use serde::{Deserialize, Serialize};
    use testify::container::Container;
    use testify::next_addr;
    use testify::wait::wait_for_tcp;

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryResp {
        status: String,
        data: Vec<String>,
    }

    #[tokio::test]
    async fn cortex_write_and_query() {
        trace_init();

        let service_addr = next_addr();

        let resp = Container::new("ubuntu/cortex", "latest")
            .with_env("TZ", "UTC")
            .with_tcp(9009, service_addr.port())
            .tail_logs(false, true)
            .run(async move {
                wait_for_tcp(service_addr).await;

                // Setup sink
                let config = format!("endpoint: http://{service_addr}/api/v1/push");
                let config: Config = serde_yaml::from_str(&config).unwrap();
                let cx = SinkContext::new_test();

                let (sink, _healthcheck) = config.build(cx).await.unwrap();
                sink.run_events(vec![Metric::gauge("foo", "", 1.1).into()])
                    .await
                    .unwrap();

                // wait until all events flushed
                tokio::time::sleep(Duration::from_secs(2)).await;

                // Query label values
                let endpoint =
                    format!("http://{service_addr}/prometheus/api/v1/label/__name__/values");
                let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();

                let req = http::Request::get(endpoint)
                    .body(Full::<Bytes>::default())
                    .unwrap();

                client.send(req).await.unwrap()
            })
            .await;

        // Assert response
        let (parts, incoming) = resp.into_parts();
        assert!(parts.status.is_success());

        let body = incoming.collect().await.unwrap().to_bytes();
        let resp = serde_json::from_slice::<QueryResp>(body.as_ref()).unwrap();

        assert_eq!(resp.status, "success".to_string());
        assert_eq!(resp.data.len(), 1);
        assert_eq!(resp.data[0], "foo");
    }
}
