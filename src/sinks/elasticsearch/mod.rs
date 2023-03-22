mod common;
mod config;
mod encoder;
mod request_builder;
mod retry;
mod service;
mod sink;

#[cfg(test)]
#[cfg(feature = "integration-tests-elasticsearch")]
mod integration_tests;
#[cfg(test)]
mod tests;

use config::DataStreamConfig;
use event::{EventRef, LogRecord};
use framework::template::{Template, TemplateParseError};
use http::uri::InvalidUri;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum BulkAction {
    Index,
    Create,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
impl BulkAction {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Index => "index",
            Self::Create => "create",
        }
    }
}

impl TryFrom<&str> for BulkAction {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "index" => Ok(BulkAction::Index),
            "create" => Ok(BulkAction::Create),
            _ => Err(format!("Invalid bulk action: {}", value)),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ElasticsearchCommonMode {
    Bulk {
        index: Template,
        action: Option<Template>,
    },
    DataStream(DataStreamConfig),
}

impl ElasticsearchCommonMode {
    fn index(&self, log: &LogRecord) -> Option<String> {
        match self {
            Self::Bulk { index, .. } => index
                .render_string(log)
                .map_err(|err| {
                    error!(
                        message = "Failed to render template for \"index\"",
                        %err,
                        drop_event = true,
                        internal_log_rate_limit = true
                    );
                })
                .ok(),
            Self::DataStream(ds) => ds.index(log),
        }
    }

    fn bulk_action<'a>(&self, event: impl Into<EventRef<'a>>) -> Option<BulkAction> {
        match self {
            ElasticsearchCommonMode::Bulk { action, .. } => match action {
                Some(template) => template
                    .render_string(event)
                    .map_err(|err| {
                        error!(
                            message = "Failed to render template for \"bulk_action\"",
                            %err,
                            drop_event = true,
                        );
                    })
                    .ok()
                    .and_then(|value| BulkAction::try_from(value.as_str()).ok()),
                None => Some(BulkAction::Index),
            },
            ElasticsearchCommonMode::DataStream(_) => Some(BulkAction::Create),
        }
    }

    const fn as_data_stream_config(&self) -> Option<&DataStreamConfig> {
        match self {
            Self::DataStream(value) => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
pub(super) enum ParseError {
    #[error("Invalid host {host:?}: {err:?}")]
    InvalidHost { host: String, err: InvalidUri },
    #[error("Host {0:?} must include hostname")]
    HostMustIncludeHostname(String),
    #[error("Index template parse error: {0}")]
    IndexTemplate(TemplateParseError),
    #[error("Batch action template parse error: {0}")]
    BatchActionTemplate(TemplateParseError),
}
