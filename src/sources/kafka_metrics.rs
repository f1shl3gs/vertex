use serde::{Deserialize, Serialize};

use crate::config::{GenerateConfig, SourceDescription};

#[derive(Debug, Deserialize, Serialize)]
struct KafkaMetricsConfig {
    servers: String,
}

impl GenerateConfig for KafkaMetricsConfig {
    fn generate_config() -> String {
        "".into()
    }
}

inventory::submit! {
    SourceDescription::new::<KafkaMetricsConfig>("kafka_metrics")
}
