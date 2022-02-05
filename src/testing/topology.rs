use std::collections::HashMap;

use framework::config::{Config, ConfigDiff};
use framework::topology;

pub async fn start_topology(
    mut config: Config,
    require_healthy: impl Into<Option<bool>>,
) -> (
    topology::RunningTopology,
    tokio::sync::mpsc::UnboundedReceiver<()>,
) {
    config.health_checks.set_require_healthy(require_healthy);
    let diff = ConfigDiff::initial(&config);
    let pieces = topology::build_or_log_errors(&config, &diff, HashMap::new())
        .await
        .unwrap();
    topology::start_validate(config, diff, pieces)
        .await
        .unwrap()
}
