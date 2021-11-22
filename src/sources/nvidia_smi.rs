use std::path::PathBuf;
use futures::StreamExt;

use serde::{Deserialize, Serialize};
use tokio::process::Command;
use event::Metric;

use crate::{sources::Source, config::{
    DataType, SourceConfig, SourceContext, SourceDescription,
    default_interval, deserialize_duration, serialize_duration, GenerateConfig
}, Error};
use crate::config::ticker_from_duration;


#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NvidiaSmiConfig {
    #[serde(default = "default_interval")]
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    interval: chrono::Duration,

    #[serde(default)]
    path: Option<PathBuf>,
}

fn default_smi_path() -> Option<PathBuf> {
    Some("/usr/bin/nvidia-smi".into())
}

impl GenerateConfig for NvidiaSmiConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::to_value(Self {
            interval: default_interval(),
            path: default_smi_path()
        }).unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<NvidiaSmiConfig>("nvidia_smi")
}

#[async_trait::async_trait]
#[typetag::serde(name = "nvidia_smi")]
impl SourceConfig for NvidiaSmiConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let mut ticker = ticker_from_duration(self.interval).unwrap()
            .take_until(ctx.shutdown);

        Ok(Box::pin(async move {
            while ticker.next().await.is_some() {

            }

            Ok(())
        }))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "nvidia_smi"
    }
}

async fn run_command(path: &PathBuf) -> Result<Vec<Metric>, Error> {
    let command = format!("{}", path.to_str().unwrap());

    let mut command = Command::new(command);
    command.kill_on_drop(true);

    // Pipe out stdout to the process
    command.stdout(std::process::Stdio::piped());

    let mut child = command.spawn()?;



    Ok(vec![])
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_command() {
        let path = "/usr/bin/pwd".into();
        let result = run_command(&path).await.unwrap();

        println!("{:?}", result);
    }
}