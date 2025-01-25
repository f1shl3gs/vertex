use bytes::{Bytes, BytesMut};
use configurable::Configurable;
use event::Event;
use framework::batch::{BatchConfig, RealtimeSizeBasedDefaultBatchSettings};
use framework::config::ProxyConfig;
use framework::http::HttpClient;
use framework::sink::util::http::{BatchedHttpSink, HttpEventEncoder, HttpRetryLogic, HttpSink};
use framework::sink::util::service::RequestConfig;
use framework::sink::util::{Buffer, Compression};
use framework::tls::TlsConfig;
use framework::{Healthcheck, HealthcheckError, Sink};
use futures_util::{FutureExt, SinkExt};
use http::header::CONTENT_TYPE;
use http::Request;
use http_body_util::{BodyExt, Full};
use serde::{Deserialize, Serialize};

/// Forward traces to jaeger collector's HTTP API.
///
/// See https://www.jaegertracing.io/docs/1.31/apis/#thrift-over-http-stable
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpSinkConfig {
    /// The HTTP address to connect to.
    #[configurable(required)]
    endpoint: String,

    #[serde(default)]
    batch: BatchConfig<RealtimeSizeBasedDefaultBatchSettings>,

    #[serde(default)]
    request: RequestConfig,

    tls: Option<TlsConfig>,
}

impl HttpSinkConfig {
    pub fn build(&self, proxy: ProxyConfig) -> framework::Result<(Sink, Healthcheck)> {
        let request_settings = self.request.into_settings();
        let client = HttpClient::new(self.tls.as_ref(), &proxy)?;
        let batch = self.batch.into_batch_settings()?;

        let sink = BatchedHttpSink::with_logic(
            self.clone(),
            Buffer::new(batch.size, Compression::None),
            HttpRetryLogic,
            request_settings,
            batch.timeout,
            client.clone(),
        )
        .sink_map_err(|err| {
            error!(message = "Error sending spans", %err);
        });

        let healthcheck = healthcheck(client, self.endpoint.clone()).boxed();

        Ok((Sink::from_event_sink(sink), healthcheck))
    }
}

pub struct JaegerEventEncoder {}

impl HttpEventEncoder<BytesMut> for JaegerEventEncoder {
    fn encode_event(&mut self, event: Event) -> Option<BytesMut> {
        let trace = event.into_trace();
        jaeger::agent::serialize_binary_batch(trace.into())
            .map_err(|err| {
                warn!(
                    message = "Encode batch failed",
                    %err,
                    internal_log_rate_limit = true
                );
            })
            .map(|data| BytesMut::from(data.as_slice()))
            .ok()
    }
}

impl HttpSink for HttpSinkConfig {
    type Input = BytesMut;
    type Output = BytesMut;
    type Encoder = JaegerEventEncoder;

    fn build_encoder(&self) -> Self::Encoder {
        JaegerEventEncoder {}
    }

    async fn build_request(&self, events: Self::Output) -> framework::Result<Request<Bytes>> {
        let req = Request::post(&self.endpoint)
            .header(CONTENT_TYPE, "application/vnd.apache.thrift.binary")
            .body(events.freeze())?;

        Ok(req)
    }
}

#[derive(Deserialize)]
struct HealthcheckResponse {
    status: String,
}

/// Request collector's / endpoint to obtain health status
///
/// See https://www.jaegertracing.io/docs/1.6/deployment/#collectors
pub async fn healthcheck(client: HttpClient, endpoint: String) -> framework::Result<()> {
    let req = Request::get(endpoint).body(Full::default())?;

    let resp = client.send(req).await?;
    let (parts, incoming) = resp.into_parts();
    let status = parts.status;
    if !status.is_success() {
        return Err(HealthcheckError::UnexpectedStatus(status).into());
    }

    let data = incoming.collect().await?.to_bytes();
    let resp: HealthcheckResponse = serde_json::from_slice(&data)?;
    if resp.status != "Server available" {
        return Err(format!("unexpected status {}", resp.status).into());
    }

    Ok(())
}
