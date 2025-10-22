use std::time::Duration;

use configurable::configurable_component;
use framework::batch::{BatchConfig, SinkBatchSettings};
use framework::config::{InputType, SinkConfig, SinkContext};
use framework::http::HttpClient;
use framework::sink::Compression;
use framework::sink::http::{HttpService, http_response_retry_logic};
use framework::sink::service::{RequestConfig, ServiceBuilderExt};
use framework::template::Template;
use framework::tls::TlsConfig;
use framework::{Healthcheck, Sink};
use http::Uri;
use tower::ServiceBuilder;

use super::health;
use super::service::InfluxdbHttpRequestBuilder;
use super::sink::InfluxdbSink;

#[derive(Clone, Copy, Debug, Default)]
struct DefaultBatchSetting;

impl SinkBatchSettings for DefaultBatchSetting {
    const MAX_EVENTS: Option<usize> = Some(4096);
    const MAX_BYTES: Option<usize> = None;
    const TIMEOUT: Duration = Duration::from_secs(1);
}

fn default_endpoint() -> String {
    "http://localhost:8086".into()
}

/// Send metrics to InfluxDB v2 with HTTP write API
///
/// See https://docs.influxdata.com/influxdb/v2/api/#operation/PostWrite
#[configurable_component(sink, name = "influxdb")]
struct Config {
    /// The endpoint to send data to.
    ///
    /// This should be a full HTTP URI, including the scheme, host, and port
    #[serde(default = "default_endpoint")]
    endpoint: String,

    #[serde(default)]
    batch: BatchConfig<DefaultBatchSetting>,

    tls: Option<TlsConfig>,

    /// Compression for HTTP requests.
    #[serde(default)]
    compression: Compression,

    #[serde(default)]
    request: RequestConfig,

    /// The organization to write data to.
    ///
    /// API Token cannot be used across organizations, so org do not need to be a `Template`.
    org: String,

    /// The bucket to write to.
    bucket: Template,

    /// API token for write authorization.
    token: String,
}

#[async_trait::async_trait]
#[typetag::serde(name = "influxdb")]
impl SinkConfig for Config {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let client = HttpClient::new(self.tls.as_ref(), &cx.proxy)?;
        let endpoint = format!("{}/api/v2/write", self.endpoint).parse::<Uri>()?;
        let batch = self.batch.into_batcher_settings()?;

        let http_request_builder = InfluxdbHttpRequestBuilder::new(
            endpoint.to_string(),
            self.token.clone(),
            self.request.header_map()?,
            self.compression,
        );
        let http_service = HttpService::new(client.clone(), http_request_builder);
        let service = ServiceBuilder::new()
            .settings(self.request.settings(), http_response_retry_logic())
            .service(http_service);

        let sink = InfluxdbSink::new(self.bucket.clone(), batch, self.compression, service);
        let healthcheck = health::healthcheck(client, self.endpoint.clone(), self.token.clone());

        Ok((Sink::Stream(Box::new(sink)), Box::pin(healthcheck)))
    }

    fn input_type(&self) -> InputType {
        InputType::metric()
    }
}
