use std::time::Duration;

use bytes::Bytes;
use configurable::configurable_component;
use event::{EventStatus, Events, Finalizable};
use framework::batch::{BatchConfig, SinkBatchSettings};
use framework::config::{DataType, ProxyConfig, SinkConfig, SinkContext};
use framework::http::HttpClient;
use framework::sink::util::Compression;
use framework::sink::util::builder::SinkBuilderExt;
use framework::stream::BatcherSettings;
use framework::{HealthcheckError, Sink, StreamSink};
use futures::{FutureExt, StreamExt, stream::BoxStream};
use http::{Method, Request, Uri};
use http_body_util::{BodyExt, Full};
use proto::log_data_body::Content;
use proto::log_report_service_client::LogReportServiceClient;
use proto::{JsonLog, LogData, LogDataBody};
use tonic::transport::Channel;

pub mod proto {
    #![allow(unused_qualifications)]

    include!(concat!(env!("OUT_DIR"), "/skywalking.v3.rs"));
}

#[derive(Clone, Debug, Default)]
struct SkyWalkingBatchSettings;

impl SinkBatchSettings for SkyWalkingBatchSettings {
    const MAX_EVENTS: Option<usize> = Some(50);
    const MAX_BYTES: Option<usize> = Some(2 * 1024 * 1024);
    const TIMEOUT: Duration = Duration::from_secs(1);
}

#[configurable_component(source, name = "skywalking")]
struct Config {
    /// The endpoint of SkyWalking
    endpoint: String,

    service: String,

    service_instance: String,

    #[serde(default)]
    compression: Compression,

    #[serde(default)]
    batch: BatchConfig<SkyWalkingBatchSettings>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "skywalking")]
impl SinkConfig for Config {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, framework::Healthcheck)> {
        let uri = Uri::try_from(self.endpoint.as_str())?;
        let batcher_settings = self.batch.clone().into_batcher_settings()?;

        let sink = SkyWalkingSink::new(
            uri.clone(),
            self.compression,
            self.service.clone(),
            self.service_instance.clone(),
            batcher_settings,
        )
        .await?;
        let uri = cx.healthcheck.uri.unwrap_or(uri);
        let healthcheck = healthcheck(uri).boxed();

        Ok((Sink::Stream(Box::new(sink)), healthcheck))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }
}

pub struct SkyWalkingSink {
    service: String,
    service_instance: String,
    batcher_settings: BatcherSettings,

    client: LogReportServiceClient<Channel>,
}

impl SkyWalkingSink {
    async fn new(
        endpoint: Uri,
        compression: Compression,
        service: String,
        service_instance: String,
        batcher_settings: BatcherSettings,
    ) -> crate::Result<Self> {
        use tonic::codec::CompressionEncoding;

        let compression = match compression {
            Compression::None => None,
            Compression::Gzip(_) => Some(CompressionEncoding::Gzip),
            Compression::Zstd(_) => Some(CompressionEncoding::Zstd),
            _ => return Err("only Gzip and Zstd compression is supported".into()),
        };

        let mut client = LogReportServiceClient::connect(endpoint).await?;
        if let Some(compression) = compression {
            client = client
                .send_compressed(compression)
                .accept_compressed(compression);
        }

        Ok(Self {
            batcher_settings,
            client,
            service,
            service_instance,
        })
    }
}

#[async_trait::async_trait]
impl StreamSink for SkyWalkingSink {
    async fn run(mut self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        let input = input
            .flat_map(|events| match events {
                Events::Logs(logs) => futures::stream::iter(logs),
                _ => futures::stream::iter(vec![]),
            })
            .batched(self.batcher_settings.into_byte_size_config());

        tokio::pin!(input);

        while let Some(mut logs) = input.next().await {
            let service = self.service.clone();
            let service_instance = self.service_instance.clone();
            let finalizers = logs.take_finalizers();

            let stream = logs.into_iter().map(move |log| {
                let content = Content::Json(JsonLog {
                    json: serde_json::to_string(log.value()).unwrap(),
                });

                LogData {
                    service: service.clone(),
                    service_instance: service_instance.clone(),
                    body: Some(LogDataBody {
                        content: Some(content),
                        ..Default::default()
                    }),
                    ..Default::default()
                }
            });

            match self.client.collect(futures::stream::iter(stream)).await {
                Ok(_resp) => {
                    finalizers.update_status(EventStatus::Delivered);
                }
                Err(status) => {
                    warn!(
                        message = "collect call failed",
                        status = ?status,
                        internal_log_rate_secs = 10
                    );

                    finalizers.update_status(EventStatus::Errored);
                }
            }
        }

        Ok(())
    }
}

// See https://skywalking.apache.org/docs/main/next/en/api/health-check/
async fn healthcheck(endpoint: Uri) -> crate::Result<()> {
    let mut builder = Uri::builder();
    if let Some(scheme) = endpoint.scheme() {
        builder = builder.scheme(scheme.clone());
    }
    if let Some(authority) = endpoint.authority() {
        builder = builder.authority(authority.clone());
    }

    let uri = builder.path_and_query("/healthcheck").build()?;

    let client = HttpClient::new(None, &ProxyConfig::default())?;
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Full::<Bytes>::default())?;

    let resp = client.send(req).await?;
    let (parts, incoming) = resp.into_parts();

    if !parts.status.is_success() {
        let data = incoming.collect().await?.to_bytes();

        return Err(HealthcheckError::UnexpectedStatus(
            parts.status,
            String::from_utf8_lossy(&data).to_string(),
        )
        .into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
