use std::collections::HashMap;
use std::time::Duration;

use codecs::EncodingConfig;
use configurable::{configurable_component, Configurable};
use framework::batch::{BatchConfig, SinkBatchSettings};
use framework::config::{DataType, SinkConfig, SinkContext, UriSerde};
use framework::http::{Auth, HttpClient, MaybeAuth};
use framework::sink::util::service::RequestConfig;
use framework::sink::util::Compression;
use framework::tls::TlsConfig;
use framework::Sink;
use framework::{template::Template, Healthcheck};
use futures_util::FutureExt;
use serde::{Deserialize, Serialize};

use super::healthcheck::health_check;
use super::sink::LokiSink;

#[derive(Clone, Copy, Default, Debug)]
pub struct LokiDefaultBatchSettings;

impl SinkBatchSettings for LokiDefaultBatchSettings {
    const MAX_EVENTS: Option<usize> = Some(10_000);
    const MAX_BYTES: Option<usize> = Some(1_000_000);
    const TIMEOUT: Duration = Duration::from_secs(1);
}

#[derive(Configurable, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutOfOrderAction {
    /// Drop the event.
    #[default]
    Drop,

    /// Rewrite the timestamp of the event to the timestamp of the latest event seen by the sink.
    RewriteTimestamp,
    // /// Accept the event.
    // ///
    // /// The event is not dropped and is sent without modification.
    // ///
    // /// Requires Loki 2.4.0 or newer.
    // Accept,
}

#[configurable_component(sink, name = "loki")]
#[derive(Clone)]
pub struct Config {
    /// The base URL of the Loki instance
    #[configurable(required, format = "uri", example = "http://example.com/ingest")]
    pub endpoint: UriSerde,

    /// Configures the encoding specific sink behavior.
    pub encoding: EncodingConfig,

    /// The tenant id that's sent with every request, by default
    /// this is not required since a proxy should set this header.
    /// When running Loki locally a tenant id is not required either.
    /// Your can read more abount tenant id's at
    /// https://github.com/grafana/loki/blob/master/docs/operations/multi-tenancy.md
    ///
    /// Note: This parameter supports Vertex's template syntax, which
    /// enables you to use dynamic per-event value.
    pub tenant: Option<Template>,

    /// A set of labels that are attached to each batch of events.
    /// Both keys and values are templatable, which enables you to
    /// attach dynamic labels to events. Note: If the set of labels
    /// has high cardinality, this can cause drastic performance
    /// issues with Loki. To prevent this from happening, reduce
    /// the number of unique label keys and values.
    #[serde(default)]
    pub labels: HashMap<Template, Template>,

    /// If this is set to "true" then when labels are collected from
    /// events those fields will also get removed from the event.
    #[serde(default)]
    pub remove_label_fields: bool,

    /// If this is set to "true" then the timestamp will be removed
    /// from the evnt payload. Note the event timestamp will still be
    /// sent as metadata to Loki for indexing.
    #[serde(default = "framework::config::default_true")]
    pub remove_timestamp: bool,

    /// Some sources may generate events with timestamps that aren't
    /// in strictly chronological order. The Loki service can't
    /// accept a stream of such events. Vertex sorts events before
    /// sending them to Loki, however some late events might
    /// arrive after a batch has been sent. This option specifies
    /// what Vertex should do with those events.
    #[serde(default)]
    pub out_of_order_action: OutOfOrderAction,

    pub auth: Option<Auth>,
    pub tls: Option<TlsConfig>,

    #[serde(default = "Compression::gzip_default")]
    pub compression: Compression,

    #[serde(default)]
    pub request: RequestConfig,

    #[serde(default)]
    pub batch: BatchConfig<LokiDefaultBatchSettings>,
    #[serde(default)]
    acknowledgements: bool,
}

impl Config {
    pub fn build_client(&self, cx: SinkContext) -> crate::Result<HttpClient> {
        let client = HttpClient::new(&self.tls, cx.proxy())?;
        Ok(client)
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "loki")]
impl SinkConfig for Config {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        for label in self.labels.keys() {
            if !valid_label_name(label) {
                return Err(format!("Invalid label name {:?}", label.get_ref()).into());
            }
        }

        let client = self.build_client(cx.clone())?;

        let config = Config {
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

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
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
