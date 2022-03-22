mod util;

use std::num::NonZeroU64;
use std::time::Duration;

use buffers::{BufferConfig, BufferType, WhenFull};
use event::{Event, EventContainer};
use framework::config::{Config, SinkOuter};
use futures_util::StreamExt;
use log_schema::log_schema;
use tempfile::tempdir;
use util::{sink, source, transform};
use vertex::testing::start_topology;

fn default_max_size() -> NonZeroU64 {
    NonZeroU64::new(1024).unwrap()
}

#[tokio::test]
async fn topology_disk_buffer_flushes_on_idle() {
    let tmpdir = tempdir().unwrap();
    let event = Event::from("foo");

    let (mut in1, source1) = source();
    let transform1 = transform("", 0.0);
    let (mut out1, sink1) = sink(10);

    let mut config = Config::builder();
    config.set_data_dir(tmpdir.path());
    config.add_source("in1", source1);
    config.add_transform("t1", &["in1"], transform1);
    let mut sink1_outer = SinkOuter::new(
        // read from both the source and the transform
        vec!["in1".to_string(), "t1".to_string()],
        Box::new(sink1),
    );
    sink1_outer.buffer = BufferConfig {
        stages: vec![BufferType::Disk {
            max_size: default_max_size(),
            when_full: WhenFull::DropNewest,
        }],
    };
    config.add_sink_outer("out1", sink1_outer);

    let (topology, _crash) = start_topology(config.build().unwrap(), false).await;

    in1.send(event).await.unwrap();

    // ensure that we get the first copy of the event within a reasonably short amount of time
    // (either from the source or the transform)
    let res = tokio::time::timeout(Duration::from_secs(1), out1.next())
        .await
        .expect("timeout 1")
        .map(|events| into_message(events.into_events().next().unwrap()))
        .expect("no output");
    assert_eq!("foo", res);

    // ensure that we get the second copy of the event
    let res = tokio::time::timeout(Duration::from_secs(1), out1.next())
        .await
        .expect("timeout 2")
        .map(|events| into_message(events.into_events().next().unwrap()))
        .expect("no output");
    assert_eq!("foo", res);

    // stop the topology only after we've received both copies of the event, to ensure it wasn't
    // shutdown that flushed them
    topology.stop().await;

    // make sure there are no unexpected stragglers
    let rest = out1.collect::<Vec<_>>().await;
    assert!(rest.is_empty());
}

fn into_message(event: Event) -> String {
    event
        .as_log()
        .get_field(log_schema().message_key())
        .unwrap()
        .to_string_lossy()
}
