use std::collections::HashMap;
use std::time::Duration;

use event::encoding::EncodingConfig;
use framework::batch::{BatchConfig, SinkBatchSettings};
use framework::config::{DataType, GenerateConfig, SinkConfig, SinkContext, UriSerde};
use framework::http::{Auth, HttpClient, MaybeAuth};
use framework::sink::util::service::RequestConfig;
use framework::tls::{TlsConfig, TlsOptions, TlsSettings};
use framework::Sink;
use framework::{template::Template, Healthcheck};
use futures_util::FutureExt;
use serde::{Deserialize, Serialize};

use super::healthcheck::health_check;
use super::sink::LokiSink;

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
    const MAX_EVENTS: Option<usize> = Some(10_000);
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

    pub tenant: Option<Template>,
    #[serde(default)]
    pub labels: HashMap<Template, Template>,

    #[serde(default = "framework::config::default_false")]
    pub remove_label_fields: bool,
    #[serde(default = "framework::config::default_true")]
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
    fn generate_config() -> String {
        format!(
            r#"
# The base URL of the Loki instance
#
endpoint: http://loki.example.com:3100

# Configures the encoding specific sink behavior.
#
encoding:
  # The encoding codec used to serialize the events before outputting.
  #
  codec: json

  # Prevent the sink from encoding the specified fields.
  #
  # except_fields:
  # - foo
  # - bar.key

  # Makes the sink encode only the specified fields.
  #
  # only_fields:
  # - k01
  # - k02.k03

  # How to format event timestamps
  #
  # Availabel values:
  # rfc3339     Formats as a RFC3339 string
  # unix        Formats as a unix timestamp
  #
  # timestamp_format: rfc3339

# The tenant id that's sent with every request, by default
# this is not required since a proxy should set this header.
# When running Loki locally a tenant id is not required either.
# Your can read more abount tenant id's at
# https://github.com/grafana/loki/blob/master/docs/operations/multi-tenancy.md
#
# Note: This parameter supports Vertex's template syntax, which
# enables you to use dynamic per-event value.
#
# tenant_id: some_tenant_id

# A set of labels that are attached to each batch of events.
# Both keys and values are templatable, which enables you to
# attach dynamic labels to events. Note: If the set of labels
# has high cardinality, this can cause drastic performance
# issues with Loki. To prevent this from happening, reduce
# the number of unique label keys and values.
#
# labels:
#   foo: bar
#   another_foo: {{ .field.key }}

# If this is set to "true" then when labels are collected from
# events those fields will also get removed from the event.
#
# remote_label_fields: false

# If this is set to "true" then the timestamp will be removed
# from the evnt payload. Note the event timestamp will still be
# sent as metadata to Loki for indexing.
#
# remove_timestamp: true

# Some sources may generate events with timestamps that aren't
# in strictly chronological order. The Loki service can't
# accept a stream of such events. Vertex sorts events before
# sending them to Loki, however some late events might
# arrive after a batch has been sent. This option specifies
# what Vertex should do with those events.
#
# Availabel values:
# drop                  Drop the event, with a warning
# rewrite_timestamp     Rewrite timestamp of the event to the
#         latest timestamp that was pushed.
#
# out_of_order_action: drop

# Configures the authentication strategy
#
# auth:
{}

# Configures the sink request behavior.
#
# request:
{}

# Configures the sink batching behavior.
#
# TODO

# Configures the TLS options for outgoing connections.
#
# tls:
{}

# TODO: compression
        "#,
            Auth::generate_commented_with_indent(2),
            RequestConfig::generate_commented_with_indent(2),
            TlsConfig::generate_commented_with_indent(2),
        )
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "loki")]
impl SinkConfig for LokiConfig {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
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

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<LokiConfig>();
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
