use async_trait::async_trait;
use buffers::Acker;
use event::Event;
use framework::batch::{BatchConfig, RealtimeSizeBasedDefaultBatchSettings};
use framework::config::ProxyConfig;
use framework::http::HttpClient;
use framework::sink::util::http::{BatchedHttpSink, HttpRetryLogic, HttpSink};
use framework::sink::util::service::{RequestConfig, RequestSettings};
use framework::sink::util::sink::StdServiceLogic;
use framework::sink::util::{Buffer, Compression};
use framework::tls::{MaybeTlsSettings, TlsConfig, TlsSettings};
use framework::{Healthcheck, Sink};
use futures_util::{FutureExt, SinkExt};
use http::Request;
use hyper::Body;
use jaeger::agent::BufferClient;

/// Forward traces to jaeger collector's HTTP API.
///
/// See https://www.jaegertracing.io/docs/1.31/apis/#thrift-over-http-stable
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknow_fields)]
pub struct HttpSinkConfig {
    endpoint: String,

    #[serde(default)]
    batch: BatchConfig<RealtimeSizeBasedDefaultBatchSettings>,

    #[serde(default)]
    request: RequestConfig,

    tls: Option<TlsConfig>,
}

impl HttpSinkConfig {
    pub fn build(
        &self,
        proxy: ProxyConfig,
        acker: Acker,
    ) -> framework::Result<(Sink, Healthcheck)> {
        let request_settings = self.request.unwrap_with(&RequestConfig::default());
        let tls = MaybeTlsSettings::from_config(&self.tls, false)?;
        let client = HttpClient::new(tls, &proxy)?;
        let batch = self.batch.into_batch_settings()?;

        let sink = BatchedHttpSink::with_logic(
            self,
            Buffer::new(batch.size, Compression::None),
            HttpRetryLogic::default(),
            request_settings,
            batch.timeout,
            client.clone(),
            acker,
            StdServiceLogic::default(),
        )
        .sink_map_err(|err| {
            error!(message = "Error sending spans", ?err);
        });

        let healthcheck = healthcheck(client.clone(), "".to_string()).boxed();

        Ok((Sink::Stream(Box::new(sink)), healthcheck))
    }
}

#[async_trait]
impl HttpSink for HttpSinkConfig {
    type Input = Vec<u8>;
    type Output = Vec<u8>;

    fn encode_event(&self, event: Event) -> Option<Self::Input> {
        let trace = event.into_trace();
        jaeger::agent::serialize_binary_batch(trace.into())
            .map_err(|err| {
                warn!(
                    message = "Encode batch failed",
                    ?err,
                    internal_log_rate_secs = 10
                );
            })
            .ok()
    }

    async fn build_request(&self, events: Self::Output) -> framework::Result<Request<Vec<u8>>> {
        let req = Request::post(&self.endpoint)
            .header("Content-Type", "application/vnd.apache.thrift.binary")
            .body(events)?;

        Ok(req)
    }
}

pub async fn healthcheck(client: HttpClient, uri: String) -> framework::Result<()> {
    // TODO
    Ok(())
}
