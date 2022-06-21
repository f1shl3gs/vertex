use framework::config::{
    DataType, GenerateConfig, Output, SourceConfig, SourceContext, SourceDescription,
};
use framework::Source;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MongodbConfig {
    endpoints: Vec<String>,
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
    async fn build(&self, _cx: SourceContext) -> crate::Result<Source> {
        todo!()
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        "mongodb"
    }
}
