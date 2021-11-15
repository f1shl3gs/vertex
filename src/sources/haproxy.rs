use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::config::{deserialize_duration, serialize_duration, default_interval, SourceConfig, SourceContext, DataType, SourceDescription, GenerateConfig};
use crate::sources::Source;
use crate::tls::TLSConfig;


#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HaproxyConfig {
    #[serde(default = "default_interval")]
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    interval: chrono::Duration,

    uri: String,

    #[serde(default)]
    tls: Option<TLSConfig>
}

#[async_trait::async_trait]
#[typetag::serde(name = "haproxy")]
impl SourceConfig for HaproxyConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        todo!()
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "haproxy"
    }
}

impl GenerateConfig for HaproxyConfig {
    fn generate_config() -> Value {
        serde_yaml::to_value(Self {
            interval: default_interval(),
            uri: "http://127.0.0.1:8404/metrics".to_string(),
            tls: None
        }).unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<HaproxyConfig>("haproxy")
}

