use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

static LOG_SCHEMA: OnceCell<LogSchema> = OnceCell::new();

lazy_static::lazy_static! {
    static ref LOG_SCHEMA_DEFAULT: LogSchema = LogSchema::default();
}

/// Components should use global `LogSchema` returned by this function. The
/// returned value can differ from `LogSchema::default()` which is unchanging.
pub fn log_schema() -> &'static LogSchema {
    LOG_SCHEMA.get().unwrap_or(&LOG_SCHEMA_DEFAULT)
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LogSchema {
    #[serde(default = "default_message_key")]
    message_key: String,
    #[serde(default = "default_timestamp_key")]
    timestamp_key: String,
    #[serde(default = "default_host_key")]
    host_key: String,
    #[serde(default = "default_source_type_key")]
    source_type_key: String,
}

fn default_message_key() -> String {
    "message".into()
}

fn default_timestamp_key() -> String {
    "timestamp".into()
}

fn default_host_key() -> String {
    "host".into()
}

fn default_source_type_key() -> String {
    "source_type".into()
}

impl LogSchema {
    pub fn message_key(&self) -> &str {
        &self.message_key
    }

    pub fn timestamp_key(&self) -> &str {
        &self.timestamp_key
    }

    pub fn host_key(&self) -> &str {
        &self.host_key
    }

    pub fn source_type_key(&self) -> &str {
        &self.source_type_key
    }
}

impl Default for LogSchema {
    fn default() -> Self {
        Self {
            message_key: default_message_key(),
            timestamp_key: default_timestamp_key(),
            host_key: default_host_key(),
            source_type_key: default_source_type_key(),
        }
    }
}