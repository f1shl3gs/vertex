use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use event::encoding::EncodingConfig;

use crate::batch::{BatchConfig, SinkBatchSettings};
use crate::config::{DataType, GenerateConfig, HealthCheck, SinkConfig, SinkContext, UriWithAuth};
use crate::http::{Auth, HttpClient};
use crate::sinks::Sink;
use crate::sinks::util::service::RequestConfig;
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
enum OutOfOrderAction {
    Drop,
    RewriteTimestamp,
}

impl Default for OutOfOrderAction {
    fn default() -> Self {
        Self::Drop
    }
}


#[derive(Clone, Debug, Deserialize, Serialize)]
struct LokiConfig {
    pub endpoint: UriWithAuth,
    pub encoding: EncodingConfig<Encoding>,

    pub tenant_id: Option<Template>,
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
            endpoint: "http://localhost:3100".try_into().unwrap(),
            encoding: EncodingConfig::from(Encoding::Json),
            tenant_id: None,
            labels: Default::default(),
            remove_label_fields: false,
            remove_timestamp: false,
            out_of_order_action: Default::default(),
            auth: None,
            request: (),
            batch: Default::default(),
            tls: None,
        }).unwrap()
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "loki")]
impl SinkConfig for LokiConfig {
    async fn build(&self, ctx: SinkContext) -> crate::Result<(Sink, HealthCheck)> {}

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
        // [a-ZA-Z0-9_]*
        let label_trim = label.get_ref().trim();
        let mut label_chars = label_trim.chars();
        if let Some(c) = label_chars.next() {
            (c.is_ascii_alphabetic() || c == '_') && label_chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_label_names() {
        let inputs = [
            "name",
            " name",
            "bee_bop",
            "a09b",
            "0ab",
            "*",
            "",
            " ",
            "{{field}}"
        ];

        for input in inputs {
            assert!(valid_label_name(input.try_into().unwrap()));
        }
    }
}