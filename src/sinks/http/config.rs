use codecs::encoding::{CharacterDelimitedEncoder, Framer, SinkType};
use codecs::{Encoder, EncodingConfigWithFraming};
use configurable::configurable_component;
use framework::batch::{BatchConfig, RealtimeSizeBasedDefaultBatchSettings};
use framework::config::{InputType, SinkConfig, SinkContext, serde_http_method, serde_uri};
use framework::http::{Auth, HttpClient};
use framework::sink::Compression;
use framework::sink::http::{HttpService, http_response_retry_logic};
use framework::sink::service::{RequestConfig, ServiceBuilderExt};
use framework::tls::TlsConfig;
use framework::{Healthcheck, HealthcheckError, Sink};
use futures::{FutureExt, future};
use http::{Method, Request, StatusCode, Uri};
use http_body_util::{BodyExt, Full};
use tower::ServiceBuilder;

use super::encoder::HttpEncoder;
use super::request_builder::HttpRequestBuilder;
use super::service::HttpSinkRequestBuilder;
use super::sink::HttpSink;

fn default_method() -> Method {
    Method::POST
}

/// Configuration for the `http` sink
#[configurable_component(sink, name = "http")]
#[serde(deny_unknown_fields)]
struct Config {
    /// The full URI to make HTTP requests to.
    #[serde(with = "serde_uri")]
    endpoint: Uri,

    #[serde(default = "default_method", with = "serde_http_method")]
    method: Method,

    /// Http auth
    auth: Option<Auth>,

    tls: Option<TlsConfig>,

    #[serde(default)]
    compression: Compression,

    #[serde(default)]
    batch: BatchConfig<RealtimeSizeBasedDefaultBatchSettings>,

    #[serde(default)]
    request: RequestConfig,

    #[serde(flatten)]
    encoding: EncodingConfigWithFraming,

    #[serde(default)]
    acknowledgements: bool,
}

#[async_trait::async_trait]
#[typetag::serde(name = "http")]
impl SinkConfig for Config {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let batch_settings = self.batch.validate()?.into_batcher_settings()?;
        let (framer, serializer) = self.encoding.build(SinkType::StreamBased);
        let encoder = Encoder::<Framer>::new(framer, serializer);
        let transformer = self.encoding.transformer();
        let content_type = {
            use codecs::encoding::{Framer, Serializer};

            match (encoder.serializer(), encoder.framer()) {
                (Serializer::Text(_), _) => Some("text/plain".to_owned()),
                (Serializer::Json(_), Framer::NewlineDelimited(_)) => {
                    Some("application/x-ndjson".to_owned())
                }
                (
                    Serializer::Json(_),
                    Framer::CharacterDelimited(CharacterDelimitedEncoder { delimiter: b',' }),
                ) => Some("application/json".to_owned()),
                _ => None,
            }
        };
        let content_encoding = self.compression.content_encoding();
        let client = HttpClient::new(self.tls.as_ref(), &cx.proxy)?;

        let encoder = HttpEncoder::new(encoder, transformer);
        let request_builder = HttpRequestBuilder::new(self.compression, encoder);
        let sink_request_builder = HttpSinkRequestBuilder::new(
            self.method.clone(),
            self.endpoint.clone(),
            self.auth.clone(),
            self.request.header_map()?,
            content_type,
            content_encoding,
        );

        let http_service = HttpService::new(client.clone(), sink_request_builder);
        let service = ServiceBuilder::new()
            .settings(self.request.settings(), http_response_retry_logic())
            .service(http_service);
        let sink = HttpSink::new(service, batch_settings, request_builder);
        let healthcheck = match &cx.healthcheck.uri {
            None => future::ok(()).boxed(),
            Some(uri) => healthcheck(uri.clone(), self.auth.clone(), client).boxed(),
        };

        Ok((Sink::Stream(Box::new(sink)), healthcheck))
    }

    fn input_type(&self) -> InputType {
        InputType::log()
    }

    fn acknowledgements(&self) -> bool {
        self.acknowledgements
    }
}

async fn healthcheck(uri: Uri, auth: Option<Auth>, client: HttpClient) -> crate::Result<()> {
    let mut req = Request::head(uri).body(Full::default())?;
    if let Some(auth) = auth {
        auth.apply(&mut req);
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
