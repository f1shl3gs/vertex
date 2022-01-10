mod concurrency;
mod map;

// re-export
pub use concurrency::*;

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::adaptive_concurrency::AdaptiveConcurrencySettings;
use crate::config::{deserialize_duration_option, serialize_duration_option};

pub const CONCURRENCY_DEFAULT: Concurrency = Concurrency::None;
pub const RATE_LIMIT_DURATION_DEFAULT: Duration = Duration::from_secs(1);
pub const RATE_LIMIT_NUM_DEFAULT: u64 = u64::MAX;
pub const RETRY_ATTEMPTS_DEFAULT: usize = usize::MAX;
pub const RETRY_MAX_DURATION_DEFAULT: Duration = Duration::from_secs(3600);
pub const RETRY_INITIAL_BACKOFF_DEFAULT: Duration = Duration::from_secs(1);
pub const TIMEOUT_DEFAULT: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestConfig {
    #[serde(default)]
    #[serde(skip_serializing_if = "concurrency_is_none")]
    pub concurrency: Concurrency,
    #[serde(
        deserialize_with = "deserialize_duration_option",
        serialize_with = "serialize_duration_option"
    )]
    pub timeout: Option<Duration>,
    #[serde(
        deserialize_with = "deserialize_duration_option",
        serialize_with = "serialize_duration_option"
    )]
    pub rate_limit_duration: Option<Duration>,
    pub rate_limit_num: Option<u64>,
    pub retry_attempts: Option<usize>,
    #[serde(
        deserialize_with = "deserialize_duration_option",
        serialize_with = "serialize_duration_option"
    )]
    pub retry_max_duration: Option<Duration>,
    #[serde(
        deserialize_with = "deserialize_duration_option",
        serialize_with = "serialize_duration_option"
    )]
    pub retry_initial_backoff: Option<Duration>,
    #[serde(default)]
    pub adaptive_concurrency: AdaptiveConcurrencySettings,
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self::new(CONCURRENCY_DEFAULT)
    }
}

impl RequestConfig {
    pub const fn new(concurrency: Concurrency) -> Self {
        Self {
            concurrency,
            timeout: Some(TIMEOUT_DEFAULT),
            rate_limit_duration: Some(RATE_LIMIT_DURATION_DEFAULT),
            rate_limit_num: Some(RATE_LIMIT_NUM_DEFAULT),
            retry_attempts: Some(RETRY_ATTEMPTS_DEFAULT),
            retry_max_duration: Some(RETRY_MAX_DURATION_DEFAULT),
            retry_initial_backoff: Some(RETRY_INITIAL_BACKOFF_DEFAULT),
            adaptive_concurrency: AdaptiveConcurrencySettings::const_default(),
        }
    }
}
