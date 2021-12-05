mod concurrency;
mod map;

use chrono::Duration;
use serde::{Deserialize, Serialize};
pub use concurrency::*;
use super::adaptive_concurrency::AdaptiveConcurrencySettings;


#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestConfig {
    #[serde(default)]
    #[serde(skip_serializing_if = "concurrency_is_none")]
    pub concurrency: Concurrency,
    pub timeout: Option<Duration>,
    pub rate_limit_duration: Option<Duration>,
    pub rate_limit_num: Option<u64>,
    pub retry_attempts: Option<usize>,
    pub retry_max_duration: Option<u64>,
    pub retry_initial_backoff: Option<Duration>,
    #[serde(default)]
    pub adaptive_concurrency: AdaptiveConcurrencySettings,
}