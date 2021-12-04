use serde::{Deserialize, Serialize};

use crate::config::{GenerateConfig, SourceDescription};


#[derive(Debug, Deserialize, Serialize)]
struct KafkaMetricsConfig {
    servers: String,
}

impl GenerateConfig for KafkaMetricsConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::to_value(Self {
            servers: "10.32.1.10:9092".to_string()
        }).unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<KafkaMetricsConfig>("kafka_metrics")
}