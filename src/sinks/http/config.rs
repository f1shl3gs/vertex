use async_trait::async_trait;
use codecs::encoding::{CharacterDelimitedEncoder, Framer, SinkType};
use codecs::{Encoder, EncodingConfigWithFraming};
use configurable::configurable_component;
use framework::batch::{BatchConfig, RealtimeSizeBasedDefaultBatchSettings};
use framework::config::{serde_http_method, serde_uri, DataType, SinkConfig, SinkContext};
use framework::http::{Auth, HttpClient};
use framework::sink::util::http::{http_response_retry_logic, HttpService};
use framework::sink::util::service::{RequestConfig, ServiceBuilderExt};
use framework::sink::util::Compression;
use framework::tls::TlsConfig;
use framework::{Healthcheck, HealthcheckError, Sink};
use futures::{future, FutureExt};
use http::{Method, Request, StatusCode, Uri};
use hyper::Body;
use tower::ServiceBuilder;

use super::encoder::HttpEncoder;
use super::request_builder::HttpRequestBuilder;
use super::service::HttpSinkRequestBuilder;
use super::sink::HttpSink;

/// Configuration for the `http` sink
#[configurable_component(sink, name = "http")]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[configurable(required)]
    #[serde(with = "serde_http_method")]
    pub method: Method,

    /// The full URI to make HTTP requests to.
    #[configurable(required)]
    #[serde(with = "serde_uri")]
    pub uri: Uri,

    /// Http auth
    pub auth: Option<Auth>,

    pub tls: Option<TlsConfig>,

    #[serde(default)]
    pub compression: Compression,

    #[serde(default)]
    pub batch: BatchConfig<RealtimeSizeBasedDefaultBatchSettings>,

    #[serde(default)]
    pub request: RequestConfig,

    #[serde(flatten)]
    #[configurable(required)]
    pub encoding: EncodingConfigWithFraming,

    pub acknowledgements: bool,
}

#[async_trait]
#[typetag::serde(name = "http")]
impl SinkConfig for Config {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let batch_settings = self.batch.validate()?.into_batcher_settings()?;
        let (framer, serializer) = self.encoding.build(SinkType::MessageBased);
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
        let client = HttpClient::new(&self.tls, &cx.proxy)?;

        let encoder = HttpEncoder::new(encoder, transformer);
        let request_builder = HttpRequestBuilder::new(self.compression, encoder);
        let sink_request_builder = HttpSinkRequestBuilder::new(
            self.method.clone(),
            self.uri.clone(),
            self.auth.clone(),
            self.request.header_map()?,
            content_type,
            content_encoding,
        );

        let http_service = HttpService::new(client.clone(), sink_request_builder);
        let service = ServiceBuilder::new()
            .settings(self.request.into_settings(), http_response_retry_logic())
            .service(http_service);
        let sink = HttpSink::new(service, batch_settings, request_builder);
        let healthcheck = match &cx.healthcheck.uri {
            None => future::ok(()).boxed(),
            Some(uri) => healthcheck(uri.clone(), self.auth.clone(), client).boxed(),
        };

        Ok((Sink::Stream(Box::new(sink)), healthcheck))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn acknowledgements(&self) -> bool {
        self.acknowledgements
    }
}

async fn healthcheck(uri: Uri, auth: Option<Auth>, client: HttpClient) -> crate::Result<()> {
    let mut req = Request::head(uri).body(Body::empty())?;
    if let Some(auth) = auth {
        auth.apply(&mut req);
    }

    let resp = client.send(req).await?;

    match resp.status() {
        StatusCode::OK => Ok(()),
        status => Err(HealthcheckError::UnexpectedStatus(status).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
