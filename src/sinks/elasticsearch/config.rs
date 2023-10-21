use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use codecs::encoding::Transformer;
use configurable::{configurable_component, Configurable};
use event::log::{OwnedValuePath, Value};
use event::{event_path, EventRef, LogRecord};
use framework::batch::{BatchConfig, RealtimeSizeBasedDefaultBatchSettings};
use framework::config::{skip_serializing_if_default, DataType, SinkConfig, SinkContext};
use framework::http::HttpClient;
use framework::sink::util::service::{RequestConfig, ServiceBuilderExt};
use framework::sink::util::Compression;
use framework::template::Template;
use framework::tls::TlsConfig;
use framework::{Healthcheck, Sink};
use futures::FutureExt;
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;

use super::common::ElasticsearchCommon;
use super::retry::ElasticsearchRetryLogic;
use super::service::{ElasticsearchService, HttpRequestBuilder};
use super::sink::ElasticsearchSink;
use super::{ElasticsearchCommonMode, ParseError};

/// The field name for the timestamp required by data stream mode
pub const DATA_STREAM_TIMESTAMP_KEY: &str = "@timestamp";

#[derive(Configurable, Debug, Deserialize, Serialize)]
pub struct BulkConfig {
    /// Action to use when making requests to the `Elasticsearch Bulk API`. Currently
    /// Vertex only supports `index` and `create`. `update` and `delete` actions are
    /// not supported.
    ///
    /// Bulk API: https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html
    pub action: Option<String>,

    /// Index name to write events to. template is supported.
    pub index: Option<String>,
}

impl BulkConfig {
    fn default_index() -> String {
        "vertex-%Y.%m.%d".into()
    }
}

#[derive(Configurable, Deserialize, Serialize, Debug, Eq, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ElasticsearchMode {
    #[default]
    Bulk,
    DataStream,
}

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "strategy")]
pub enum ElasticsearchAuth {
    Basic { user: String, password: String },
}

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataStreamConfig {
    /// The data stream type used to construct the data stream at index time.
    #[serde(rename = "type", default = "DataStreamConfig::default_type")]
    pub dtype: Template,

    /// The data stream dataset used to construct the data stream at index time.
    #[serde(default = "DataStreamConfig::default_dataset")]
    pub dataset: Template,

    /// The data stream namespace used to construct the data stream at index time.
    #[serde(default = "DataStreamConfig::default_namespace")]
    pub namespace: Template,

    /// Automatically routes events by deriving the data stream name using specific event
    /// field with the `data_stream.type-data_stream.dataset-data_stream.namespace` format.
    /// If enabled, the data_stream.* event fields will take precedence over the
    /// data_stream.type, data_stream.dataset, and data_stream.namespace settings, but
    /// will fall back to them if any of the fields are missing from the event.
    #[serde(default = "DataStreamConfig::default_auto_routing")]
    pub auto_routing: bool,
    #[serde(default = "DataStreamConfig::default_sync_fields")]
    pub sync_fields: bool,
}

impl Default for DataStreamConfig {
    fn default() -> Self {
        Self {
            dtype: Self::default_type(),
            dataset: Self::default_dataset(),
            namespace: Self::default_namespace(),
            auto_routing: Self::default_auto_routing(),
            sync_fields: Self::default_sync_fields(),
        }
    }
}

impl DataStreamConfig {
    fn default_type() -> Template {
        Template::try_from("logs").expect("couldn't build default type template")
    }

    fn default_dataset() -> Template {
        Template::try_from("generic").expect("couldn't build default dataset")
    }

    fn default_namespace() -> Template {
        Template::try_from("default").expect("couldn't build default namespace template")
    }

    const fn default_auto_routing() -> bool {
        true
    }

    const fn default_sync_fields() -> bool {
        true
    }

    /// If there is a `timestamp` field, rename it to the expected `@timestamp`
    /// for Elastic Common Schema.
    pub fn remap_timestamp(&self, log: &mut LogRecord) {
        // we keep it if the timestamp field is @timestamp
        let timestamp_key = log_schema().timestamp_key();
        if timestamp_key.to_string() == DATA_STREAM_TIMESTAMP_KEY {
            return;
        }

        if let Some(value) = log.remove(timestamp_key) {
            log.insert(event_path!(DATA_STREAM_TIMESTAMP_KEY), value);
        }
    }

    pub fn dtype<'a>(&self, event: impl Into<EventRef<'a>>) -> Option<String> {
        self.dtype
            .render_string(event)
            .map_err(|err| {
                error!(
                    message = "Failed to render template for \"data_stream.type\"",
                    %err,
                    drop_event = true,
                    internal_log_rate_limit = true,
                );
            })
            .ok()
    }

    pub fn dataset<'a>(&self, event: impl Into<EventRef<'a>>) -> Option<String> {
        self.dataset
            .render_string(event)
            .map_err(|err| {
                error!(
                    message = "Failed to render template for \"data_stream.dataset\"",
                    %err,
                    drop_event = true,
                    internal_log_rate_limit = true
                );
            })
            .ok()
    }

    pub fn namespace<'a>(&self, event: impl Into<EventRef<'a>>) -> Option<String> {
        self.namespace
            .render_string(event)
            .map_err(|err| {
                error!(
                    message = "Failed to render template for \"data_stream.namespace\"",
                    %err,
                    drop_event = true,
                    internal_log_rate_limit = true
                );
            })
            .ok()
    }

    pub fn sync_fields(&self, log: &mut LogRecord) {
        if !self.sync_fields {
            return;
        }

        let dtype = self.dtype(&*log);
        let dataset = self.dataset(&*log);
        let namespace = self.namespace(&*log);

        if log.as_map().is_none() {
            *log.value_mut() = Value::Object(BTreeMap::new());
        }

        let existing = log
            .as_map_mut()
            .expect("must be a map")
            .entry("data_stream".into())
            .or_insert_with(|| Value::Object(BTreeMap::new()))
            .as_object_mut()
            .unwrap();
        if let Some(dtype) = dtype {
            existing
                .entry("type".into())
                .or_insert_with(|| dtype.into());
        }
        if let Some(dataset) = dataset {
            existing
                .entry("dataset".into())
                .or_insert_with(|| dataset.into());
        }
        if let Some(namespace) = namespace {
            existing
                .entry("namespace".into())
                .or_insert_with(|| namespace.into());
        }
    }

    pub fn index(&self, log: &LogRecord) -> Option<String> {
        let (dtype, dataset, namespace) = if !self.auto_routing {
            (self.dtype(log)?, self.dataset(log)?, self.namespace(log)?)
        } else {
            let data_stream = log
                .get(event_path!("data_stream"))
                .and_then(|ds| ds.as_object());
            let dtype = data_stream
                .and_then(|ds| ds.get("type"))
                .map(|value| value.to_string_lossy().into_owned())
                .or_else(|| self.dtype(log))?;
            let dataset = data_stream
                .and_then(|ds| ds.get("dataset"))
                .map(|value| value.to_string_lossy().into_owned())
                .or_else(|| self.dataset(log))?;
            let namespace = data_stream
                .and_then(|ds| ds.get("namespace"))
                .map(|value| value.to_string_lossy().into_owned())
                .or_else(|| self.namespace(log))?;
            (dtype, dataset, namespace)
        };

        Some(format!("{}-{}-{}", dtype, dataset, namespace))
    }
}

#[configurable_component(sink, name = "elasticsearch")]
#[derive(Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The Elasticsearch endpoint to send logs to. This should be full URL.
    #[configurable(required, format = "uri")]
    pub endpoint: String,

    /// The `doc_type` for your index data, This is only relevant for Elasticsearch <= 6.X.
    /// If you are using >= 7.0 you do not need to set this optoin since Elasticsearch
    /// has removed it.
    pub doc_type: Option<String>,

    /// Stop Vertex from sending the `type` to Elasticsearch, which was deprecated in Elasticsearch
    /// 7.x and removed in Elasticsearch 8.x.
    ///
    /// If enabled the `doc_type` option will be ignored.
    #[serde(default)]
    pub suppress_type_name: bool,

    /// The name of the event key that should map to Elasticsearch's "_id" field. By
    /// default, Vertex does not set the "_id" field, which allows Elasticsearch to
    /// set this automatically. You should think carefully about setting your own
    /// Elasticsearch IDs, since this can "hinder performance".
    ///
    /// https://www.elastic.co/guide/en/elasticsearch/reference/master/tune-for-indexing-speed.html#_use_auto_generated_ids
    pub id_key: Option<OwnedValuePath>,

    /// Name of the pipeline to apply.
    pub pipeline: Option<String>,

    /// The type of index mechanism. If `data_stream` mode is enabled, the `bulk.action` is
    /// set to "create".
    #[serde(default)]
    pub mode: ElasticsearchMode,

    #[serde(default)]
    pub compression: Compression,

    #[serde(default)]
    pub batch: BatchConfig<RealtimeSizeBasedDefaultBatchSettings>,

    #[serde(default)]
    pub request: RequestConfig,
    pub auth: Option<ElasticsearchAuth>,
    pub tls: Option<TlsConfig>,
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    pub encoding: Transformer,
    pub query: Option<HashMap<String, String>>,

    /// Options for the bulk mode.
    pub bulk: Option<BulkConfig>,

    /// Options for the data stream mode.
    pub data_stream: Option<DataStreamConfig>,
}

impl Config {
    pub fn index(&self) -> crate::Result<Template> {
        let index = self
            .bulk
            .as_ref()
            .and_then(|c| c.index.as_deref())
            .map(String::from)
            .unwrap_or_else(BulkConfig::default_index);
        Ok(Template::try_from(index.as_str()).map_err(ParseError::IndexTemplate)?)
    }

    pub fn bulk_action(&self) -> crate::Result<Option<Template>> {
        Ok(self
            .bulk
            .as_ref()
            .and_then(|c| c.action.as_deref())
            .map(|value| Template::try_from(value).map_err(ParseError::BatchActionTemplate))
            .transpose()?)
    }

    pub fn common_mode(&self) -> crate::Result<ElasticsearchCommonMode> {
        match self.mode {
            ElasticsearchMode::Bulk => {
                let index = self.index()?;
                let action = self.bulk_action()?;

                Ok(ElasticsearchCommonMode::Bulk { index, action })
            }
            ElasticsearchMode::DataStream => Ok(ElasticsearchCommonMode::DataStream(
                self.data_stream.clone().unwrap_or_default(),
            )),
        }
    }
}

#[async_trait]
#[typetag::serde(name = "elasticsearch")]
impl SinkConfig for Config {
    async fn build(&self, cx: SinkContext) -> framework::Result<(Sink, Healthcheck)> {
        let common = ElasticsearchCommon::parse_config(self).await?;
        let http_client = HttpClient::new(&self.tls, cx.proxy())?;
        let batch_settings = self.batch.into_batcher_settings()?;
        let request_limits = self.request.unwrap_with(&RequestConfig::default());
        let http_request_builder = HttpRequestBuilder {
            bulk_uri: common.bulk_uri.clone(),
            request_config: self.request.clone(),
            http_auth: common.http_auth.clone(),
            query_params: common.query_params.clone(),
            compression: self.compression,
        };

        let service = ServiceBuilder::new()
            .settings(request_limits, ElasticsearchRetryLogic)
            .service(ElasticsearchService::new(http_client, http_request_builder));

        let sink = ElasticsearchSink {
            batch_settings,
            request_builder: common.request_builder.clone(),
            transformer: self.encoding.clone(),
            service,
            mode: common.mode.clone(),
            id_key_field: self.id_key.clone(),
        };

        let client = HttpClient::new(&self.tls, cx.proxy())?;
        let healthcheck = common.healthcheck(client).boxed();

        Ok((Sink::Stream(Box::new(sink)), healthcheck))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }

    #[test]
    fn default_data_stream_config() {
        let _d = DataStreamConfig::default();
    }

    #[test]
    fn parse_mode() {
        let config = serde_yaml::from_str::<Config>(
            r#"
endpoint: ""
mode: data_stream
data_stream:
    type: synthetics
"#,
        )
        .unwrap();

        assert!(matches!(config.mode, ElasticsearchMode::DataStream));
        assert!(config.data_stream.is_some())
    }
}
