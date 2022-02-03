use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::{
    default_interval, deserialize_duration, serialize_duration, DataType, GenerateConfig, Output,
    SourceConfig, SourceContext, SourceDescription,
};
use crate::sources::Source;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MongodbConfig {
    endpoints: Vec<String>,
    #[serde(default = "default_interval")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    interval: Duration,
}

inventory::submit! {
    SourceDescription::new::<MongodbConfig>("mongodb")
}

impl GenerateConfig for MongodbConfig {
    fn generate_config() -> String {
        r#"
# The endpoint to MongoDB server.
endpoints:
- localhost:8500

# The interval between scrapes.
#
# interval: 15s
"#
        .into()
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "mongodb")]
impl SourceConfig for MongodbConfig {
    async fn build(&self, _ctx: SourceContext) -> crate::Result<Source> {
        todo!()
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        "mongodb"
    }
}
