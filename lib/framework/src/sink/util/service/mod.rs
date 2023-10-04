mod concurrency;
mod map;

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::Arc;

// re-export
pub use concurrency::*;
pub use map::Map;

use std::time::Duration;

use configurable::Configurable;
use serde::{Deserialize, Serialize};
use tower::layer::util::Stack;
use tower::limit::RateLimit;
use tower::retry::Retry;
use tower::timeout::Timeout;
use tower::{Layer, Service, ServiceBuilder};

use super::adaptive_concurrency::AdaptiveConcurrencySettings;
use crate::batch::Batch;
use crate::sink::util::adaptive_concurrency::service::AdaptiveConcurrencyLimit;
use crate::sink::util::adaptive_concurrency::AdaptiveConcurrencyLimitLayer;
use crate::sink::util::retries::{FixedRetryPolicy, RetryLogic};
use crate::sink::util::service::map::MapLayer;
use crate::sink::util::sink::{BatchSink, PartitionBatchSink, Response};

pub const CONCURRENCY_DEFAULT: Concurrency = Concurrency::None;
pub const RATE_LIMIT_DURATION_DEFAULT: Duration = Duration::from_secs(1);
pub const RATE_LIMIT_NUM_DEFAULT: u64 = u64::MAX;
pub const RETRY_ATTEMPTS_DEFAULT: usize = usize::MAX;
pub const RETRY_MAX_DURATION_DEFAULT: Duration = Duration::from_secs(3600);
pub const RETRY_INITIAL_BACKOFF_DEFAULT: Duration = Duration::from_secs(1);
pub const TIMEOUT_DEFAULT: Duration = Duration::from_secs(60);

pub type Svc<S, L> = RateLimit<AdaptiveConcurrencyLimit<Retry<FixedRetryPolicy<L>, Timeout<S>>, L>>;
pub type BatchedSink<S, B, RL> = BatchSink<Svc<S, RL>, B>;
pub type PartitionSink<S, B, RL, K> = PartitionBatchSink<Svc<S, RL>, B, K>;

/// Middleware settings for outbound requests.
///
/// Various settings can be configured, such as concurrency and rate limits, timeouts, etc.
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestConfig {
    #[serde(default)]
    #[serde(skip_serializing_if = "concurrency_is_none")]
    pub concurrency: Concurrency,

    /// The time a request can take before being aborted.
    ///
    /// It is highly recommended that you do not lower this value below the service’s
    /// internal timeout, as this could create orphaned requests, pile on retries, and
    /// result in duplicate data downstream.
    #[serde(with = "humanize::duration::serde_option")]
    pub timeout: Option<Duration>,

    /// The time window used for the `rate_limit_num` option.
    #[serde(with = "humanize::duration::serde_option")]
    pub rate_limit_duration: Option<Duration>,

    /// The maximum number of requests allowed within the `rate_limit_duration_secs` time window.
    pub rate_limit_num: Option<u64>,

    /// The maximum number of retries to make for failed requests.
    ///
    /// The default, for all intents and purposes, represents an infinite number of retries.
    pub retry_attempts: Option<usize>,

    /// The maximum amount of time to wait between retries.
    #[serde(with = "humanize::duration::serde_option")]
    pub retry_max_duration: Option<Duration>,

    /// The amount of time to wait before attempting the first retry for a failed request.
    ///
    /// After the first retry has failed, the fibonacci sequence will be used to select
    /// future backoffs.
    #[serde(with = "humanize::duration::serde_option")]
    pub retry_initial_backoff: Option<Duration>,

    #[serde(default)]
    pub adaptive_concurrency: AdaptiveConcurrencySettings,

    /// Headers that will be added to the request.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self::new(CONCURRENCY_DEFAULT)
    }
}

impl RequestConfig {
    pub fn new(concurrency: Concurrency) -> Self {
        Self {
            concurrency,
            timeout: Some(TIMEOUT_DEFAULT),
            rate_limit_duration: Some(RATE_LIMIT_DURATION_DEFAULT),
            rate_limit_num: Some(RATE_LIMIT_NUM_DEFAULT),
            retry_attempts: Some(RETRY_ATTEMPTS_DEFAULT),
            retry_max_duration: Some(RETRY_MAX_DURATION_DEFAULT),
            retry_initial_backoff: Some(RETRY_INITIAL_BACKOFF_DEFAULT),
            adaptive_concurrency: AdaptiveConcurrencySettings::const_default(),
            headers: Default::default(),
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

    pub fn batch_sink<B, RL, S>(
        &self,
        retry_logic: RL,
        service: S,
        batch: B,
        batch_timeout: Duration,
    ) -> BatchedSink<S, B, RL>
    where
        RL: RetryLogic<Response = S::Response>,
        S: Service<B::Output> + Clone + Send + 'static,
        S::Error: Into<crate::Error> + Send + Sync + 'static,
        S::Response: Send + Response,
        S::Future: Send + 'static,
        B: Batch,
        B::Output: Send + Clone + 'static,
    {
        let service = ServiceBuilder::new()
            .settings(self.clone(), retry_logic)
            .service(service);

        BatchSink::new(service, batch, batch_timeout)
    }
}

pub trait ServiceBuilderExt<L> {
    fn map<R1, R2, F>(self, f: F) -> ServiceBuilder<Stack<MapLayer<R1, R2>, L>>
    where
        F: Fn(R1) -> R2 + Send + Sync + 'static;

    fn settings<RL, R>(
        self,
        settings: RequestSettings,
        retry_logic: RL,
    ) -> ServiceBuilder<Stack<RequestLayer<RL, R>, L>>;
}

impl<L> ServiceBuilderExt<L> for ServiceBuilder<L> {
    fn map<R1, R2, F>(self, f: F) -> ServiceBuilder<Stack<MapLayer<R1, R2>, L>>
    where
        F: Fn(R1) -> R2 + Send + Sync + 'static,
    {
        self.layer(MapLayer::new(Arc::new(f)))
    }

    fn settings<RL, R>(
        self,
        settings: RequestSettings,
        retry_logic: RL,
    ) -> ServiceBuilder<Stack<RequestLayer<RL, R>, L>> {
        self.layer(RequestLayer {
            settings,
            retry_logic,
            _req: PhantomData,
        })
    }
}

#[derive(Clone, Debug)]
pub struct RequestLayer<L, R> {
    settings: RequestSettings,
    retry_logic: L,
    _req: PhantomData<R>,
}

impl<S, RL, R> Layer<S> for RequestLayer<RL, R>
where
    S: Service<R> + Send + Clone + 'static,
    S::Response: Send + 'static,
    S::Error: Into<crate::Error> + Send + Sync + 'static,
    S::Future: Send + 'static,
    RL: RetryLogic<Response = S::Response> + Send + 'static,
    R: Clone + Send + 'static,
{
    type Service = Svc<S, RL>;

    fn layer(&self, inner: S) -> Self::Service {
        let policy = self.settings.retry_policy(self.retry_logic.clone());

        ServiceBuilder::new()
            .rate_limit(
                self.settings.rate_limit_num,
                self.settings.rate_limit_duration,
            )
            .layer(AdaptiveConcurrencyLimitLayer::new(
                self.settings.concurrency,
                self.settings.adaptive_concurrency,
                self.retry_logic.clone(),
            ))
            .retry(policy)
            .timeout(self.settings.timeout)
            .service(inner)
    }
}
