use futures_util::StreamExt;
use std::collections::{BTreeMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use tail::provider::Provider;
use tokio_stream::wrappers::IntervalStream;

use crate::config::{
    deserialize_std_duration, serialize_std_duration, DataType, GenerateConfig, SourceConfig,
    SourceContext, SourceDescription,
};
use crate::sources::Source;

#[derive(Debug, Deserialize, Serialize)]
struct TailConfig {
    #[serde(default = "default_ignore_older_than")]
    #[serde(
        deserialize_with = "deserialize_std_duration",
        serialize_with = "serialize_std_duration"
    )]
    ignore_older_than: Duration,

    host_key: Option<String>,

    include: Vec<PathBuf>,
    #[serde(default)]
    exclude: Vec<PathBuf>,

    #[serde(default = "default_glob_interval")]
    #[serde(
        deserialize_with = "deserialize_std_duration",
        serialize_with = "serialize_std_duration"
    )]
    glob_interval: Duration,
}

fn default_ignore_older_than() -> Duration {
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
        let data_dir = ctx.global.make_subdir(&ctx.name)?;

        // states
        let paths = Arc::new(Mutex::new(BTreeMap::new()));

        // scan path
        let glob = tail::provider::Glob::new(&self.include, &self.exclude).unwrap(); // TODO: handle error properly

        // re-scan:
        let shutdown = ctx.shutdown.clone();
        let interval = tokio::time::interval(self.glob_interval);
        let mut ticker = IntervalStream::new(interval).take_until(shutdown);
        let tailings = Arc::clone(&paths);
        tokio::spawn(async move {
            while ticker.next().await.is_some() {
                for path in glob.scan() {
                    if tailings.lock().unwrap().contains_key(&path) {
                        continue;
                    }

                    // TODO: try tailing, if success insert path and timestamp

                    info!(message = "Tailing new file", ?path);

                    tailings.lock().unwrap().insert(path, Instant::now());
                }
            }
        });

        Ok(Box::pin(async {
            tokio::time::sleep(tokio::time::Duration::from_secs(10 * 60)).await;
            Ok(())
        }))
    }

    fn output_type(&self) -> DataType {
        DataType::Log
    }

    fn source_type(&self) -> &'static str {
        "tail"
    }
}
