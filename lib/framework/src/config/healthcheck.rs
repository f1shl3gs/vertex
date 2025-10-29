use serde::{Deserialize, Serialize};

/// Healthcheck options
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct HealthcheckOptions {
    /// Whether healthcheck are enabled for all sinks
    ///
    /// Can be overridden on a per-sink basis.
    pub enabled: bool,

    /// Whether to require a sink to report as being healthy during startup.
    ///
    /// When enabled and a sink reports not being healthy, Vertex will exit during
    /// start-up.
    pub require_healthy: bool,
}

impl Default for HealthcheckOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            require_healthy: false,
        }
    }
}

impl HealthcheckOptions {
    pub fn set_require_healthy(&mut self, require_healthy: impl Into<Option<bool>>) {
        if let Some(require_healthy) = require_healthy.into() {
            self.require_healthy = require_healthy;
        }
    }

    pub fn merge(&mut self, other: Self) {
        self.enabled &= other.enabled;
        self.require_healthy |= other.require_healthy;
    }
}
