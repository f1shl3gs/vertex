use serde::{Deserialize, Serialize};
use crate::sinks::util::service::Concurrency;

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Action {
    // Above the given limit, additional requests will return with an error
    Defer,
    // Above the given limit, additional requests will be silently dropped
    Drop,
}

impl Default for Action {
    fn default() -> Self {
        Self::Defer
    }
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize)]
struct LimitParams {
    // The scale is the amount a request's dealy increases at higher levels of the variable.
    #[serde(default)]
    scale: f64,

    // The knee is the point above which a request's delay increase at an exponential scale
    // rather than a linear scale.
    knee_start: Option<usize>,

    knee_exp: Option<f64>,

    // The limit is the level above which more requests will be denied.
    limit: Option<usize>,

    // The action specifies how over-limit requests will be denied.
    #[serde(default)]
    action: Action
}

impl LimitParams {
    fn action_at_level(&self, level: usize) -> Option<Action> {
        self.limit
            .and_then(|limit| (level > limit).then(|| self.action))
    }

    fn scale(&self, level: usize) -> f64 {
        ((level - 1) as f64).mul_add(
            self.scale,
            self.knee_start
                .map(|knee| {
                    self.knee_exp
                        .unwrap_or_else(|| self.scale + 1.0)
                        .powf(level.saturating_sub(knee) as f64) - 1.0
                })
                .unwrap_or(0.0)
        )
    }
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize)]
struct TestParams {
    // The number of requests to issue.
    requests: usize,

    // The time interval between requests.
    #[serde(default = "default_interval")]
    interval: f64,

    // The delay is the base time every request takes return.
    delay: f64,

    // The jitter is the amount of per-request response time randomness, as a fraction of `delay`.
    // The average response time will be `delay * (1 + jitter)` and will have an exponential
    // distribution with λ=1.
    #[serde(default)]
    jitter: f64,

    #[serde(default)]
    concurrency_limit_params: LimitParams,

    #[serde(default)]
    rate: LimitParams,

    #[serde(default = "default_concurrency")]
    concurrency: Concurrency,
}

const fn default_interval() -> f64 { 0.0 }

const fn default_concurrency() -> Concurrency {
    Concurrency::Adaptive
}
