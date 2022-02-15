use std::fmt::Debug;

use async_trait::async_trait;
use framework::config::{DataType, Output, SourceConfig, SourceContext};
use framework::Source;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct InternalTracesConfig {}

#[async_trait]
#[typetag::serde(name = "internal_traces")]
impl SourceConfig for InternalTracesConfig {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        todo!()
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Trace)]
    }

    fn source_type(&self) -> &'static str {
        "internal_traces"
    }
}
