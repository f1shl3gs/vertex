use tokio::time::{Duration, timeout};

use crate::config::Config;
use crate::topology::test::{GenerateLogSource, NullSinkConfig, start_topology};

#[tokio::test]
async fn sources_finished() {
    let mut old_config = Config::builder();
    let source = GenerateLogSource { count: 1 };
    old_config.add_source("in", source);
    old_config.add_sink("out", &["in"], NullSinkConfig {});

    let (topology, _crash) = start_topology(old_config.compile().unwrap(), false).await;

    timeout(Duration::from_secs(2), topology.sources_finished())
        .await
        .unwrap();
}
