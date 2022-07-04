use std::collections::BTreeMap;

use event::log::Value;
use event::{EventRef, LogRecord};
use framework::batch::{BatchConfig, RealtimeSizeBasedDefaultBatchSettings};
use framework::config::{DataType, SinkConfig, SinkContext};
use framework::http::HttpClient;
use framework::sink::util::service::{RequestConfig, ServiceBuilderExt};
use framework::sink::util::{Compression, Transformer};
use framework::template::Template;
use framework::tls::TlsConfig;
use framework::{Healthcheck, Sink};
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;

use crate::sinks::elasticsearch::common::ElasticsearchCommon;
use crate::sinks::elasticsearch::retry::ElasticsearchRetryLogic;
use crate::sinks::elasticsearch::service::{ElasticsearchService, HttpRequestBuilder};
use crate::sinks::elasticsearch::sink::ElasticsearchSink;
use crate::sinks::elasticsearch::{ElasticsearchCommonMode, ParseError};

/// The field name for the timestamp required by data stream mode
pub const DATA_STREAM_TIMESTAMP_KEY: &str = "@timestamp";

#[derive(Debug, Deserialize, Serialize)]
pub struct BulkConfig {
    action: Option<String>,
    index: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub enum ElasticsearchMode {
    Bulk,
    DataStream,
}

impl Default for ElasticsearchMode {
    fn default() -> Self {
        Self::Bulk
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ElasticsearchAuth {
    Basic { user: String, password: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DataStreamConfig {
    #[serde(rename = "type", default = "DataStreamConfig::default_type")]
    pub dtype: Template,
    #[serde(default = "DataStreamConfig::default_dataset")]
    pub dataset: Template,
    #[serde(default = "DataStreamConfig::default_namespace")]
    pub namespace: Template,
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

    pub fn remap_timestamp(&self, log: &mut LogRecord) {
        // we keep it if the timestamp field is @timestamp
        let timestamp_key = log_schema().timestamp_key();
        if timestamp_key == DATA_STREAM_TIMESTAMP_KEY {
            return;
        }

        if let Some(value) = log.remove_field(timestamp_key) {
            log.insert_field(DATA_STREAM_TIMESTAMP_KEY, value)
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
                    internal_log_rate_secs = 30,
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
                    internal_log_rate_secs = 30
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
                    internal_log_rate_secs = 30
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

        let existing = log
            .fields
            .entry("data_stream".into())
            .or_insert_with(|| Value::Object(BTreeMap::new()))
            .as_object_mut_unwrap();
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
            let data_stream = log.get_field("data_stream").and_then(|ds| ds.as_object());
            let dtype = data_stream
                .and_then(|ds| ds.get("type"))
                .map(|value| value.to_string_lossy())
                .or_else(|| self.dtype(log))?;
            let dataset = data_stream
                .and_then(|ds| ds.get("dataset"))
                .map(|value| value.to_string_lossy())
                .or_else(|| self.dataset(log))?;
            let namespace = data_stream
                .and_then(|ds| ds.get("namespace"))
                .map(|value| value.to_string_lossy())
                .or_else(|| self.namespace(log))?;
            (dtype, dataset, namespace)
        };

        Some(format!("{}-{}-{}", dtype, dataset, namespace))
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ElasticsearchConfig {
    pub endpoint: String,

    pub doc_type: Option<String>,
    pub suppress_type_name: bool,
    pub id_key: Option<String>,
    pub pipeline: Option<String>,
    #[serde(default)]
    pub mode: ElasticsearchMode,
    #[serde(default)]
    pub compression: Compression,
    pub batch: BatchConfig<RealtimeSizeBasedDefaultBatchSettings>,
    pub request: RequestConfig,
    pub auth: Option<ElasticsearchAuth>,
    pub tls: Option<TlsConfig>,
    #[serde(
        skip_serializing_if = "crate::serde::skip_serializing_if_default",
        default
    )]
    pub encoding: Transformer,

    pub bulk: Option<BulkConfig>,
    pub data_stream: Option<DataStreamConfig>,
}

impl ElasticsearchConfig {
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
impl SinkConfig for ElasticsearchConfig {
    async fn build(&self, cx: SinkContext) -> framework::Result<(Sink, Healthcheck)> {
        let common = ElasticsearchCommon::parse_config(self).await?;
        let http_client = HttpClient::new(common.tls_settings.clone(), cx.proxy())?;
        let batch_settings = self.batch.into_batch_settings()?;
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
            acker: cx.acker,
            mode: common.mode.clone(),
            id_key_field: self.id_key.clone(),
        };

        let client = HttpClient::new(common.tls_settings.clone(), cx.proxy())?;
        let healthcheck = common.healthcheck(client).boxed();
        let stream = framework::Sink::from_event_sink(sink);

        Ok((stream, healthcheck))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn sink_type(&self) -> &'static str {
        "elasticsearch"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_data_stream_config() {
        let _d = DataStreamConfig::default();
    }
}
