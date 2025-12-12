use std::collections::HashMap;
use std::time::Duration;

use bytes::Bytes;
use configurable::configurable_component;
use framework::batch::{BatchConfig, SinkBatchSettings};
use framework::config::{InputType, SinkConfig, SinkContext};
use framework::http::{Auth, HttpClient};
use framework::sink::service::{RequestConfig, ServiceBuilderExt};
use framework::tls::TlsConfig;
use framework::{Healthcheck, HealthcheckError, Sink};
use futures::FutureExt;
use http::uri::PathAndQuery;
use http::{Method, Request, Uri};
use http_body_util::{BodyExt, Full};
use tower::ServiceBuilder;
use value::OwnedValuePath;

use super::request_builder::AlertmanagerRequestBuilder;
use super::service::{AlertmanagerRetryLogic, AlertsService};
use super::sink::AlertmanagerSink;

#[derive(Clone, Debug, Default)]
struct SmallRealtimeBatchSettings;

impl SinkBatchSettings for SmallRealtimeBatchSettings {
    const MAX_EVENTS: Option<usize> = Some(64);
    const MAX_BYTES: Option<usize> = Some(4 * 1024 * 1024);
    const TIMEOUT: Duration = Duration::from_secs(1);
}

#[configurable_component(sink, name = "alertmanager")]
struct Config {
    /// The endpoint to send alert to.
    ///
    /// This should be a full HTTP URI, including the scheme, host and port (if necessary)
    #[serde(with = "framework::config::http::uri")]
    endpoint: Uri,

    auth: Option<Auth>,

    tls: Option<TlsConfig>,

    #[serde(default)]
    request: RequestConfig,

    #[serde(default)]
    batch: BatchConfig<SmallRealtimeBatchSettings>,

    /// the key is the `LabelName` the value is the path of `LabelValue`
    #[serde(default)]
    labels: HashMap<String, OwnedValuePath>,

    /// the key is the `LabelName` the value is the path of `LabelValue`
    #[serde(default)]
    annotations: HashMap<String, OwnedValuePath>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "alertmanager")]
impl SinkConfig for Config {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let client = HttpClient::new(self.tls.as_ref(), &cx.proxy)?;

        let builder =
            AlertmanagerRequestBuilder::new(self.labels.clone(), self.annotations.clone());
        let service = ServiceBuilder::new()
            .settings(self.request.settings(), AlertmanagerRetryLogic)
            .service(AlertsService::new(
                client.clone(),
                self.endpoint.clone(),
                self.auth.as_ref(),
            ));
        let sink = AlertmanagerSink::new(
            self.batch.clone().into_batcher_settings()?,
            builder,
            service,
        );

        let healthcheck = healthcheck(client, self.endpoint.clone(), self.auth.clone()).boxed();

        Ok((Sink::Stream(Box::new(sink)), healthcheck))
    }

    fn input_type(&self) -> InputType {
        InputType::log()
    }
}

/// https://prometheus.io/docs/alerting/latest/management_api/
async fn healthcheck(client: HttpClient, endpoint: Uri, auth: Option<Auth>) -> crate::Result<()> {
    let mut endpoint = endpoint.into_parts();
    endpoint.path_and_query = Some(PathAndQuery::from_static("/-/healthy"));

    let mut req = Request::builder()
        .method(Method::GET)
        .uri(endpoint)
        .body(Full::<Bytes>::default())?;
    if let Some(auth) = auth {
        auth.apply(&mut req);
    }

    let resp = client.send(req).await?;
    let (parts, incoming) = resp.into_parts();
    if parts.status.is_success() {
        return Ok(());
    }

    let body = incoming
        .collect()
        .await
        .map_err(|err| HealthcheckError::UnexpectedStatus(parts.status, err.to_string()))?
        .to_bytes();

    Err(HealthcheckError::UnexpectedStatus(
        parts.status,
        String::from_utf8_lossy(&body).to_string(),
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
