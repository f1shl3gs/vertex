use std::path::Path;

use testify::next_addr;

use crate::config::Config;
use crate::topology::test::{NullSinkConfig, ResourceSourceConfig, start_topology};

#[tokio::test]
async fn topology_doesnt_reload_new_data_dir() {
    let addr = next_addr();

    let mut old_config = Config::builder();
    old_config.add_source("in", ResourceSourceConfig::new(addr));
    old_config.add_sink("out", &["in"], NullSinkConfig);
    old_config.global.data_dir = Some(Path::new("/asdf").to_path_buf());

    let mut new_config = Config::builder();
    new_config.add_source("in", ResourceSourceConfig::new(addr));
    new_config.add_sink("out", &["in"], NullSinkConfig);
    new_config.global.data_dir = Some(Path::new("/asdf").to_path_buf());
    let (mut topology, _crash) = start_topology(old_config.compile().unwrap(), false).await;

    new_config.global.data_dir = Some(Path::new("/qwerty").to_path_buf());

    topology
        .reload_config_and_respawn(new_config.compile().unwrap())
        .await
        .unwrap();

    assert_eq!(
        topology.config.global.data_dir,
        Some(Path::new("/asdf").to_path_buf())
    );
}
