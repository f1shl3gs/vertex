use futures_util::FutureExt;
use http::Uri;
use std::collections::HashMap;
use std::time::Duration;

use event::encoding::EncodingConfig;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::batch::{BatchConfig, SinkBatchSettings};
use crate::config::{DataType, GenerateConfig, HealthCheck, SinkConfig, SinkContext, UriSerde};
use crate::http::{Auth, HttpClient, MaybeAuth};
use crate::sinks::loki::healthcheck::health_check;
use crate::sinks::loki::sink::LokiSink;
use crate::sinks::util::service::{Concurrency, RequestConfig};
use crate::sinks::util::Compression;
use crate::sinks::Sink;
use crate::template::Template;
use crate::tls::{TlsConfig, TlsOptions, TlsSettings};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    Json,
    Text,
    Logfmt,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct LokiDefaultBatchSettings;

impl SinkBatchSettings for LokiDefaultBatchSettings {
    const MAX_EVENTS: Option<usize> = Some(100_00);
    const MAX_BYTES: Option<usize> = Some(1_000_000);
    const TIMEOUT: Duration = Duration::from_secs(1);
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutOfOrderAction {
    Drop,
    RewriteTimestamp,
}

impl Default for OutOfOrderAction {
    fn default() -> Self {
        Self::Drop
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LokiConfig {
    pub endpoint: UriSerde,
    pub encoding: EncodingConfig<Encoding>,
    #[serde(default)]
    pub compression: Compression,

    pub tenant: Option<Template>,
    pub labels: HashMap<Template, Template>,

    #[serde(default = "crate::config::default_false")]
    pub remove_label_fields: bool,
    #[serde(default = "crate::config::default_true")]
    pub remove_timestamp: bool,
    #[serde(default)]
    pub out_of_order_action: OutOfOrderAction,

    pub auth: Option<Auth>,

    #[serde(default)]
    pub request: RequestConfig,

    #[serde(default)]
    pub batch: BatchConfig<LokiDefaultBatchSettings>,

    pub tls: Option<TlsOptions>,
}

impl LokiConfig {
    pub fn build_client(&self, cx: SinkContext) -> crate::Result<HttpClient> {
        let tls = TlsSettings::from_options(&self.tls)?;
        let client = HttpClient::new(tls, cx.proxy())?;
        Ok(client)
    }
}

impl GenerateConfig for LokiConfig {
    fn generate_config() -> Value {
        serde_yaml::to_value(Self {
            endpoint: "http://localhost:3100".parse().unwrap(),
            encoding: EncodingConfig::from(Encoding::Json),
            tenant: None,
            labels: Default::default(),
            remove_label_fields: false,
            remove_timestamp: false,
            out_of_order_action: Default::default(),
            auth: None,
            request: RequestConfig::new(Concurrency::None),
            batch: Default::default(),
            tls: None,
            compression: Compression::default(),
        })
        .unwrap()
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "loki")]
impl SinkConfig for LokiConfig {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, HealthCheck)> {
        if self.labels.is_empty() {
            return Err("`labels` must include at least one label".into());
        }

        for label in self.labels.keys() {
            if !valid_label_name(label) {
                return Err(format!("Invalid label name {:?}", label.get_ref()).into());
            }
        }

        let client = self.build_client(cx.clone())?;

        let config = LokiConfig {
            auth: self.auth.choose_one(&self.auth)?,
            ..self.clone()
        };

        let sink = LokiSink::new(config.clone(), client.clone(), cx)?;

        let health_check = health_check(config, client).boxed();

        Ok((Sink::Stream(Box::new(sink)), health_check))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn sink_type(&self) -> &'static str {
        "loki"
    }
}

pub fn valid_label_name(label: &Template) -> bool {
    label.is_dynamic() || {
        // Loki follows prometheus on this https://prometheus.io/docs/concepts/data_model/#metric-names-and-labels
        // Although that isn't explicityl said anywhere besides what's in the code.
        // The closest mention is in seciont about Parse Expression https://grafana.com/docs/loki/latest/logql/
        //
        // [a-zA-Z_][a-zA-Z0-9_]*
        let label_trim = label.get_ref().trim();
        let mut label_chars = label_trim.chars();
        if let Some(ch) = label_chars.next() {
            (ch.is_ascii_alphabetic() || ch == '_')
                && label_chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::test_generate_config;

    #[test]
    fn generate_config() {
        test_generate_config::<LokiConfig>();
    }

    #[test]
    fn validate_label_names() {
        let inputs = [
            ("name", true),
            (" name", true),
            ("bee_bop", true),
            ("a09b", true),
            ("{{field}}", true),
            ("0ab", false),
            ("*", false),
            ("", false),
            (" ", false),
        ];

        for (input, want) in inputs {
            let tmpl = input.try_into().unwrap();
            assert_eq!(valid_label_name(&tmpl), want, "input: {}", input);
        }
    }
}
