use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

static LOG_SCHEMA: OnceLock<LogSchema> = OnceLock::new();

/// Loads `LogSchema` from configurations and sets global schema. Once this is
/// done, configurations can be correctly loaded using configured log schema
/// defaults.
///
/// # Errors
///
/// This function will fail if the `builder` fails
///
/// # Panic
///
/// If deny is set, will panic if schema has already been set
pub fn init_log_schema<F>(builder: F, deny_if_set: bool) -> Result<(), Vec<String>>
where
    F: FnOnce() -> Result<LogSchema, Vec<String>>,
{
    let log_schema = builder()?;
    if LOG_SCHEMA.set(log_schema).is_err() && deny_if_set {
        panic!("Couldn't set schema");
    }

    Ok(())
}

static LOG_SCHEMA_DEFAULT: OnceLock<LogSchema> = OnceLock::new();

/// Components should use global `LogSchema` returned by this function. The
/// returned value can differ from `LogSchema::default()` which is unchanging.
pub fn log_schema() -> &'static LogSchema {
    LOG_SCHEMA
        .get()
        .unwrap_or(LOG_SCHEMA_DEFAULT.get_or_init(LogSchema::default))
}

#[allow(clippy::derive_partial_eq_without_eq)]
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

    /// Merge two `LogSchema` instances together.
    ///
    /// # Errors
    ///
    /// This function will fail when the `LogSchema` to be merged contains
    /// conflicting keys
    pub fn merge(&mut self, other: &LogSchema) -> Result<(), Vec<String>> {
        let mut errors = vec![];
        let default_value = LOG_SCHEMA_DEFAULT.get_or_init(LogSchema::default);

        if *other != *default_value {
            // If the set value is the defalt, override it. If it's already overridden, error
            if self.host_key() != default_value.host_key() && self.host_key() != other.host_key() {
                errors.push("conflicting values for 'log_schema.host_key' found".to_owned());
            } else {
                self.host_key = other.host_key.to_string();
            }

            if self.message_key() != default_value.message_key()
                && self.message_key() != other.message_key()
            {
                errors.push("conflicting values for 'log_schema.message_key' found".to_owned());
            } else {
                self.message_key = other.message_key.to_string();
            }

            if self.timestamp_key() != default_value.timestamp_key()
                && self.timestamp_key() != other.timestamp_key()
            {
                errors.push("conflicting values for 'log_schema.timestamp_key' found".to_owned());
            } else {
                self.timestamp_key = other.timestamp_key.to_string();
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
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
