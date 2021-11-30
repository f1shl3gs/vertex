mod controller;
mod future;
mod layer;
mod semaphore;
mod service;
mod events;
#[cfg(test)]
mod tests;

pub(crate) use layer::AdaptiveConcurrencyLimitLayer;
pub(crate) use service::AdaptiveConcurrencyLimit;

use std::time::Duration;

use serde::{Deserialize, Serialize};


pub(self) const MAX_CONCURRENCY: usize = 256;

/// The defaults for these values were chosen after running several simulations
/// on a test service that had various responses to load. The values are the best
/// balances found between competing outcomes.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdaptiveConcurrencySettings {
    /// This value maintained high concurrency without holding it too high under
    /// adverse conditions.
    pub decrease_ratio: f64,

    /// This value achieved the best balance between quick response and stability
    pub ewma_alpha: f64,

    /// This value avoided changing concurrency too aggressively when there is
    /// fluctuation in the RTT measurements.
    pub rtt_deviation_scale: f64,
}

impl Default for AdaptiveConcurrencySettings {
    fn default() -> Self {
        Self {
            decrease_ratio: 0.9,
            ewma_alpha: 0.4,
            rtt_deviation_scale: 2.5,
        }
    }
}

