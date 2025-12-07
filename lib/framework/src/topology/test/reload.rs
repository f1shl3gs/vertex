use std::net::{SocketAddr, TcpListener};
use std::time::Duration;

use buffer::{BufferConfig, BufferType, WhenFull};
use futures::StreamExt;
use testify::wait::wait_for_tcp;
use testify::{next_addr, temp_dir};
use tokio::time::sleep;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::config::{ComponentKey, Config};
use crate::topology::test::{
    GenerateLogSource, MockSinkConfig, NoopTransformConfig, NullSinkConfig, ResourceSourceConfig,
    start_topology,
};

#[tokio::test]
async fn topology_reuse_old_port() {
    let address = next_addr();

    let mut old_config = Config::builder();
    old_config.add_source("in1", ResourceSourceConfig { listen: address });
    old_config.add_sink("out", &["in1"], NullSinkConfig);

    let mut new_config = Config::builder();
    new_config.add_source("in2", ResourceSourceConfig { listen: address });
    new_config.add_sink("out", &["in2"], NullSinkConfig);

    let (mut topology, _crash) = start_topology(old_config.compile().unwrap(), false).await;
    assert!(
        topology
            .reload_config_and_respawn(new_config.compile().unwrap())
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn topology_rebuild_old() {
    let address_0 = next_addr();
    let address_1 = next_addr();

    let mut old_config = Config::builder();
    old_config.add_source("in1", ResourceSourceConfig::new(address_0));
    old_config.add_sink("out", &["in1"], NullSinkConfig);

    let mut new_config = Config::builder();
    new_config.add_source("in1", ResourceSourceConfig::new(address_1));
    new_config.add_sink("out", &["in1"], NullSinkConfig);

    // Will cause the new_config to fail on build
    let _bind = TcpListener::bind(address_1).unwrap();

    let (mut topology, _crash) = start_topology(old_config.compile().unwrap(), false).await;
    assert!(
        !topology
            .reload_config_and_respawn(new_config.compile().unwrap())
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn topology_old() {
    let address = next_addr();

    let mut old_config = Config::builder();
    old_config.add_source("in1", ResourceSourceConfig::new(address));
    old_config.add_sink("out", &["in1"], NullSinkConfig);

    let (mut topology, _crash) = start_topology(old_config.compile().unwrap(), false).await;

    let mut old_config = Config::builder();
    old_config.add_source("in1", ResourceSourceConfig::new(address));
    old_config.add_sink("out", &["in1"], NullSinkConfig);
    assert!(
        topology
            .reload_config_and_respawn(old_config.compile().unwrap())
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn topology_reuse_old_port_sink() {
    let address = next_addr();

    let source = GenerateLogSource::new(usize::MAX);
    let transform = NoopTransformConfig;

    let mut old_config = Config::builder();
    old_config.add_source("in", source.clone());
    old_config.add_transform("trans", &["in"], transform.clone());
    old_config.add_sink("out1", &["trans"], MockSinkConfig::tcp(address));

    let mut new_config = Config::builder();
    new_config.add_source("in", source.clone());
    new_config.add_transform("trans", &["in"], transform.clone());
    new_config.add_sink("out1", &["trans"], MockSinkConfig::tcp(address));

    reload_sink_test(
        old_config.compile().unwrap(),
        new_config.compile().unwrap(),
        address,
        address,
    )
    .await;
}

#[tokio::test]
async fn topology_reuse_old_port_cross_dependency() {
    // Reload with source that uses address of changed sink.
    let address_0 = next_addr();
    let address_1 = next_addr();

    let transform = NoopTransformConfig;

    let mut old_config = Config::builder();
    old_config.add_source("in", GenerateLogSource::new(usize::MAX));
    old_config.add_transform("trans", &["in"], transform.clone());
    old_config.add_sink("out1", &["trans"], MockSinkConfig::tcp(address_0));

    let mut new_config = Config::builder();
    new_config.add_source("in", ResourceSourceConfig::new(address_0));
    new_config.add_transform("trans", &["in"], transform.clone());
    new_config.add_sink("out1", &["trans"], MockSinkConfig::tcp(address_1));

    reload_sink_test(
        old_config.compile().unwrap(),
        new_config.compile().unwrap(),
        address_0,
        address_1,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn topology_disk_buffer_conflict() {
    let address_0 = next_addr();
    let address_1 = next_addr();
    let data_dir = temp_dir();
    std::fs::create_dir(&data_dir).unwrap();

    let mut old_config = Config::builder();
    old_config.global.data_dir = Some(data_dir.clone());
    old_config.add_source("in", GenerateLogSource::new(usize::MAX));
    old_config.add_transform("trans", &["in"], NoopTransformConfig);
    old_config.add_sink("out", &["trans"], MockSinkConfig::tcp(address_0));
    old_config.sinks[&ComponentKey::from("out")].buffer = BufferConfig {
        when_full: WhenFull::Block,
        typ: BufferType::Disk {
            max_size: 1024,
            max_record_size: 1024,
            max_chunk_size: 1024,
        },
    };

    let mut new_config = Config::builder();
    old_config.global.data_dir = Some(data_dir);
    old_config.add_source("in", GenerateLogSource::new(usize::MAX));
    old_config.add_transform("trans", &["in"], NoopTransformConfig);
    old_config.add_sink("out", &["trans"], MockSinkConfig::tcp(address_1));
    new_config.sinks[&ComponentKey::from("out")].buffer = BufferConfig {
        when_full: WhenFull::Block,
        typ: BufferType::Disk {
            max_size: 1024,
            max_record_size: 1024,
            max_chunk_size: 1024,
        },
    };

    reload_sink_test(
        old_config.compile().unwrap(),
        new_config.compile().unwrap(),
        address_0,
        address_1,
    )
    .await;
}

async fn reload_sink_test(
    old_config: Config,
    new_config: Config,
    old_address: SocketAddr,
    new_address: SocketAddr,
) {
    let (mut topology, crash) = start_topology(old_config, false).await;
    let mut crash_stream = UnboundedReceiverStream::new(crash);

    // Wait for sink to come online
    wait_for_tcp(old_address).await;

    // Give topology some time to run
    sleep(Duration::from_secs(1)).await;

    assert!(
        topology
            .reload_config_and_respawn(new_config)
            .await
            .unwrap()
    );

    // Give old time to shutdown if it didn't, and new one to come online.
    sleep(Duration::from_secs(2)).await;

    tokio::select! {
        _ = wait_for_tcp(new_address) => {}//Success
        _ = crash_stream.next() => panic!(),
    }
}
