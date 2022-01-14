use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::config::{
    deserialize_duration, serialize_duration, DataType, GenerateConfig, Output, SourceConfig,
    SourceContext, SourceDescription,
};
use crate::sources::Source;

#[derive(Debug, Deserialize, Serialize)]
struct TailConfig {
    #[serde(default = "default_ignore_older_than")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    ignore_older_than: Duration,

    host_key: Option<String>,

    include: Vec<PathBuf>,
    exclude: Vec<PathBuf>,

    #[serde(default = "default_glob_interval")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    glob_interval: Duration,
}

fn default_ignore_older_than() -> Duration {
    // 12 hours
    Duration::from_secs(12 * 60 * 60)
}

fn default_glob_interval() -> Duration {
    Duration::from_secs(3)
}

impl GenerateConfig for TailConfig {
    fn generate_config() -> Value {
        serde_yaml::to_value(Self {
            ignore_older_than: default_ignore_older_than(),
            host_key: None,
            include: vec!["/path/to/include/*.log".into()],
            exclude: vec!["/path/to/exclude/noop.log".into()],
            glob_interval: default_glob_interval(),
        })
        .unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<TailConfig>("tail")
}

#[async_trait::async_trait]
#[typetag::serde(name = "tail")]
impl SourceConfig for TailConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        // add the source name as a subdir, so that multiple sources can operate
        // within the same given data_dir(e.g. the global one) without the file
        // servers' checkpointers interfering with each other
        let data_dir = ctx.globals.make_subdir(&ctx.key.id())?;

        todo!()
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn source_type(&self) -> &'static str {
        "tail"
    }
}
