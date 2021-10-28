use std::path::{Path, PathBuf};

use chrono::Duration;
use serde::{Deserialize, Serialize};

use crate::config::{deserialize_duration, serialize_duration};

#[derive(Debug, Deserialize, Serialize)]
struct TailConfig {
    #[serde(default = "default_ignore_older_than")]
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_with")]
    ignore_older_than: Duration,

    host_key: Option<String>,

    include: Vec<PathBuf>,
    exclude: Vec<PathBuf>,

    #[serde(default = "default_glob_interval")]
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_with")]
    glob_interval: Duration,
}

fn default_ignore_older_than() -> Duration {
    Duration::hours(12)
}

fn default_glob_interval() -> Duration {
    Duration::seconds(3)
}