use std::path::PathBuf;

use chrono::Duration;
use serde::{Deserialize, Serialize};

use crate::config::{DataType, deserialize_duration, serialize_duration, SourceConfig, SourceContext};
use crate::sources::Source;

#[derive(Debug, Deserialize, Serialize)]
struct TailConfig {
    #[serde(default = "default_ignore_older_than")]
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    ignore_older_than: Duration,

    host_key: Option<String>,

    include: Vec<PathBuf>,
    exclude: Vec<PathBuf>,

    #[serde(default = "default_glob_interval")]
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    glob_interval: Duration,
}

fn default_ignore_older_than() -> Duration {
    Duration::hours(12)
}

fn default_glob_interval() -> Duration {
    Duration::seconds(3)
}

#[async_trait::async_trait]
#[typetag::serde(name = "tail")]
impl SourceConfig for TailConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        // add the source name as a subdir, so that multiple sources can operate
        // within the same given data_dir(e.g. the global one) without the file
        // servers' checkpointers interfering with each other
        let data_dir = ctx.global
            .make_subdir(&ctx.name)?;


        todo!()
    }

    fn output_type(&self) -> DataType {
        DataType::Log
    }

    fn source_type(&self) -> &'static str {
        "tail"
    }
}
