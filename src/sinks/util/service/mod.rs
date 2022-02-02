mod concurrency;
mod map;

// re-export
pub use concurrency::*;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tower::limit::RateLimit;
use tower::retry::Retry;
use tower::timeout::Timeout;
use tower::{Service, ServiceBuilder};

use super::adaptive_concurrency::AdaptiveConcurrencySettings;
use crate::config::{deserialize_duration_option, serialize_duration_option, GenerateConfig};
use crate::sinks::util::adaptive_concurrency::service::AdaptiveConcurrencyLimit;
use crate::sinks::util::adaptive_concurrency::AdaptiveConcurrencyLimitLayer;
use crate::sinks::util::retries::{FixedRetryPolicy, RetryLogic};
use crate::sinks::util::sink::Response;

pub const CONCURRENCY_DEFAULT: Concurrency = Concurrency::None;
pub const RATE_LIMIT_DURATION_DEFAULT: Duration = Duration::from_secs(1);
pub const RATE_LIMIT_NUM_DEFAULT: u64 = u64::MAX;
pub const RETRY_ATTEMPTS_DEFAULT: usize = usize::MAX;
pub const RETRY_MAX_DURATION_DEFAULT: Duration = Duration::from_secs(3600);
pub const RETRY_INITIAL_BACKOFF_DEFAULT: Duration = Duration::from_secs(1);
pub const TIMEOUT_DEFAULT: Duration = Duration::from_secs(60);

pub type Svc<S, L> = RateLimit<AdaptiveConcurrencyLimit<Retry<FixedRetryPolicy<L>, Timeout<S>>, L>>;

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

impl GenerateConfig for RequestConfig {
    fn generate_config() -> String {
        r#"
# The maximum number of in-flight requests allowed at any given time,
# or “adaptive” to allow Vertex to automatically set the limit based
# on current network and service conditions.
concurrency: 128

# The maximum time a request can take before being aborted. It is highly
# recommended that you do not lower this value below the service’s internal
# timeout, as this could create orphaned requests, pile on retries, and
# result in duplicate data downstream.
timeout: 30s

# The time window, used for the rate_limit_num option.
# rate_limit_duration: 1s

# The maximum number of requests allowed within the "rate_limit_duration",
# time window.
# rate_limit_num: 512

# The maximum number of retries to make for failed requests. The default,
# for all intents and purposes, represents an infinite number of retries
retry_attempts: 3

# The maximum amount of time to wait between retries.
retry_max_duration: 30s

# The amount of time to wait before attempting the first retry for a
# failed request. Once, the first retry has failed the fibonacci sequence
# will be used to select future backoffs.
retry_initial_backoff: 1s
"#
        .into()
    }
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

    pub fn unwrap_with(&self, defaults: &Self) -> RequestSettings {
        RequestSettings {
            concurrency: self.concurrency.parse_concurrency(defaults.concurrency),
            timeout: self.timeout.unwrap_or(TIMEOUT_DEFAULT),
            rate_limit_duration: self
                .rate_limit_duration
                .unwrap_or(RATE_LIMIT_DURATION_DEFAULT),
            rate_limit_num: self
                .rate_limit_num
                .or(defaults.rate_limit_num)
                .unwrap_or(RATE_LIMIT_NUM_DEFAULT),
            retry_attempts: self
                .retry_attempts
                .or(defaults.retry_attempts)
                .unwrap_or(RETRY_ATTEMPTS_DEFAULT),
            retry_max_duration: self
                .retry_max_duration
                .or(defaults.retry_max_duration)
                .unwrap_or(RETRY_MAX_DURATION_DEFAULT),
            retry_initial_backoff: self
                .retry_initial_backoff
                .or(defaults.retry_initial_backoff)
                .unwrap_or(RETRY_INITIAL_BACKOFF_DEFAULT),
            adaptive_concurrency: self.adaptive_concurrency,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RequestSettings {
    pub concurrency: Option<usize>,
    pub timeout: Duration,
    pub rate_limit_duration: Duration,
    pub rate_limit_num: u64,
    pub retry_attempts: usize,
    pub retry_max_duration: Duration,
    pub retry_initial_backoff: Duration,
    pub adaptive_concurrency: AdaptiveConcurrencySettings,
}

impl RequestSettings {
    pub fn retry_policy<L: RetryLogic>(&self, logic: L) -> FixedRetryPolicy<L> {
        FixedRetryPolicy::new(
            self.retry_attempts,
            self.retry_initial_backoff,
            self.retry_max_duration,
            logic,
        )
    }

    pub fn service<RL, S, R>(&self, retry: RL, service: S) -> Svc<S, RL>
    where
        RL: RetryLogic<Response = S::Response>,
        S: Service<R> + Clone + Send + 'static,
        S::Error: Into<crate::Error> + Send + Sync + 'static,
        S::Response: Send + Response,
        S::Future: Send + 'static,
        R: Send + Clone + 'static,
    {
        let policy = self.retry_policy(retry.clone());

        ServiceBuilder::new()
            .rate_limit(self.rate_limit_num, self.rate_limit_duration)
            .layer(AdaptiveConcurrencyLimitLayer::new(
                self.concurrency,
                self.adaptive_concurrency,
                retry,
            ))
            .retry(policy)
            .timeout(self.timeout)
            .service(service)
    }
}
