use serde::{Deserialize, Serialize};
use crate::batch::{BatchConfig, RealtimeSizeBasedDefaultBatchSettings};

#[derive(Debug, Deserialize, Serialize)]
pub struct BulkConfig {
    action: Option<String>,
    index: Option<String>
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub enum ElasticsearchMode {
    Bulk,
    DataStream,
}

impl Default for ElasticsearchMode {
    fn default() -> Self {
        Self::Bulk
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ElasticsearchConfig {
    pub endpoint: String,

    pub doc_type: Option<String>,
    pub id_key: Option<String>,
    pub pipeline: Option<String>,
    #[serde(default)]
    pub mode: ElasticsearchMode,
    // #[serde(default)]
    // pub compression: Compression,

    pub batch: BatchConfig<RealtimeSizeBasedDefaultBatchSettings>
}