use chrono::Duration;

use super::aggregate::Mode;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MultilineConfig {
    pub start_pattern: String,
    pub condition_pattern: String,
    pub timeout: Duration,
    pub mode: Mode
}

impl TryFrom<&MultilineConfig> for line

