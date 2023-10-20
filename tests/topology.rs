mod util;

use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::atomic::AtomicUsize;
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;

use buffers::{BufferConfig, BufferType, WhenFull};
use event::array::into_event_stream;
use event::{Event, EventContainer, Events, LogRecord};
use framework::config::{Config, SinkOuter};
use framework::{topology, Pipeline};
use futures_util::{stream, StreamExt};
use log_schema::log_schema;
use tempfile::tempdir;
use tokio::time::sleep;
use util::{
    sink, sink_failing_healthcheck, sink_with_data, source, source_with_data, start_topology,
    trace_init, transform, MockSourceConfig,
};

fn into_message(event: Event) -> String {
    event
        .as_log()
        .get_field(log_schema().message_key())
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

fn into_message_stream(events: Events) -> impl futures::Stream<Item = String> {
    stream::iter(events.into_events().map(into_message))
}

fn basic_config() -> Config {
    let mut config = Config::builder();
    config.add_source("in1", source().1);
    config.add_sink("out1", &["in1"], sink(10).1);
    config.build().unwrap()
}

fn basic_config_with_sink_failing_healthcheck() -> Config {
    let mut config = Config::builder();
    config.add_source("in1", source().1);
    config.add_sink("out1", &["in1"], sink_failing_healthcheck(10).1);
    config.build().unwrap()
}

#[tokio::test]
async fn shutdown_while_active() {
    let source_event_counter = Arc::new(AtomicUsize::new(0));
    let source_event_total = source_event_counter.clone();

    let (mut in1, rx) = Pipeline::new_with_buffer(10);

    let source1 = MockSourceConfig::new_with_event_counter(rx, source_event_counter);
    let transform1 = transform(" transformed", 0.0);
    let (out1, sink1) = sink(10);

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_transform("t1", &["in1"], transform1);
    config.add_sink("out1", &["t1"], sink1);

    let (topology, _crash) = start_topology(config.build().unwrap(), false).await;
    let pump_handle = tokio::spawn(async move {
        let mut stream = futures::stream::repeat(Event::from("test"));
        in1.send_event_stream(&mut stream).await
    });

    // Wait until at least 100 events have been seen by the source so we know the pump is running
    // and pushing events through the pipeline.
    while source_event_total.load(Ordering::SeqCst) < 100 {
        sleep(Duration::from_millis(10)).await;
    }

    // Now shut down the RunningTopology while Events are still beging processed.
    let stop_complete = tokio::spawn(topology.stop());

    // Now that shutdown has begun we should be able to drain the Sink without
    // blocking forever, as the source should shut down and close its output channel.
    let processed_events = out1.collect::<Vec<_>>().await;
    assert_eq!(
        processed_events
            .iter()
            .fold(0, |acc, events| { acc + events.len() }),
        source_event_total.load(Ordering::Relaxed)
    );

    for event in processed_events.into_iter().flat_map(Events::into_events) {
        assert_eq!(
            event
                .as_log()
                .get_field(log_schema().message_key())
                .unwrap()
                .to_string_lossy(),
            "test transformed".to_string()
        );
    }

    stop_complete.await.unwrap();

    // We expect the pump to fail with an error since we shutdown the
    // source it was sending to while it was running.
    assert!(pump_handle.await.unwrap().is_err());
}

#[tokio::test]
async fn topology_source_and_sink() {
    let (mut in1, source1) = source();
    let (out1, sink1) = sink(10);

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_sink("out1", &["in1"], sink1);

    let (topology, _crash) = start_topology(config.build().unwrap(), false).await;

    let event = Event::from("this");
    in1.send(event.clone()).await.unwrap();

    topology.stop().await;

    let res = out1.flat_map(into_event_stream).collect::<Vec<_>>().await;

    assert_eq!(vec![event], res);
}

#[tokio::test]
async fn topology_multiple_sources() {
    let (mut in1, source1) = source();
    let (mut in2, source2) = source();
    let (mut out1, sink1) = sink(10);

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_source("in2", source2);
    config.add_sink("out1", &["in1", "in2"], sink1);

    let (topology, _crash) = start_topology(config.build().unwrap(), false).await;

    let event1 = Event::from("this");
    let event2 = Event::from("that");

    in1.send(event1.clone()).await.unwrap();

    let out_event1 = out1.next().await;

    in2.send(event2.clone()).await.unwrap();

    let out_event2 = out1.next().await;

    topology.stop().await;

    assert_eq!(out_event1, Some(event1.into()));
    assert_eq!(out_event2, Some(event2.into()));
}

#[tokio::test]
async fn topology_multiple_sinks() {
    trace_init();

    // Create source #1 as `in1`, sink #1, and sink #2, with both sink #1 and sink #2 attached to `in1`.
    let (mut in1, source1) = source();
    let (out1, sink1) = sink(10);
    let (out2, sink2) = sink(10);

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_sink("out1", &["in1"], sink1);
    config.add_sink("out2", &["in1"], sink2);

    let (topology, _crash) = start_topology(config.build().unwrap(), false).await;

    // Send an event into source #1:
    let event = Event::from("this");
    in1.send(event.clone()).await.unwrap();

    // Drop the inputs to the two sources, which will ensure they drain all items and stop
    // themselves, and also fully stop the topology:
    drop(in1);
    topology.stop().await;

    let res1 = out1.flat_map(into_event_stream).collect::<Vec<_>>().await;
    let res2 = out2.flat_map(into_event_stream).collect::<Vec<_>>().await;

    // We should see that both sinks got the exact same event:
    let expected = vec![event];
    assert_eq!(expected, res1);
    assert_eq!(expected, res2);
}

#[tokio::test]
async fn topology_transform_chain() {
    let (mut in1, source1) = source();
    let transform1 = transform(" first", 0.0);
    let transform2 = transform(" second", 0.0);
    let (out1, sink1) = sink(10);

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_transform("t1", &["in1"], transform1);
    config.add_transform("t2", &["t1"], transform2);
    config.add_sink("out1", &["t2"], sink1);

    let (topology, _crash) = start_topology(config.build().unwrap(), false).await;

    let event = Event::from("this");

    in1.send(event).await.unwrap();

    topology.stop().await;

    let res = out1.flat_map(into_message_stream).collect::<Vec<_>>().await;

    assert_eq!(vec!["this first second"], res);
}

#[tokio::test]
async fn topology_remove_one_source() {
    trace_init();

    let (mut in1, source1) = source();
    let (mut in2, source2) = source();
    let (_out1, sink1) = sink(10);

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_source("in2", source2);
    config.add_sink("out1", &["in1", "in2"], sink1);

    let (mut topology, _crash) = start_topology(config.build().unwrap(), false).await;

    let (out1, sink1) = sink(10);

    let mut config = Config::builder();
    config.add_source("in1", source().1);
    config.add_sink("out1", &["in1"], sink1);

    assert!(topology
        .reload_config_and_respawn(config.build().unwrap())
        .await
        .unwrap());

    // Send an event into both source #1 and source #2:
    let event1 = Event::from("this");
    let event2 = Event::from("that");
    let h_out1 = tokio::spawn(out1.flat_map(into_event_stream).collect::<Vec<_>>());

    in1.send(event1.clone()).await.unwrap();
    in2.send(event2.clone()).await.unwrap_err();

    // Drop the inputs to the two sources, which will ensure they drain all items and stop
    // themselves, and also fully stop the topology:
    drop(in1);
    drop(in2);
    topology.stop().await;

    let res = h_out1.await.unwrap();
    assert_eq!(vec![event1], res);
}

#[tokio::test]
async fn topology_remove_one_sink() {
    let (mut in1, source1) = source();
    let (out1, sink1) = sink(10);
    let (out2, sink2) = sink(10);

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_sink("out1", &["in1"], sink1);
    config.add_sink("out2", &["in1"], sink2);

    let (mut topology, _crash) = start_topology(config.build().unwrap(), false).await;

    let mut config = Config::builder();
    config.add_source("in1", source().1);
    config.add_sink("out1", &["in1"], sink(10).1);

    assert!(topology
        .reload_config_and_respawn(config.build().unwrap())
        .await
        .unwrap());

    let event = Event::from("this");

    in1.send(event.clone()).await.unwrap();

    topology.stop().await;

    let res1 = out1.flat_map(into_event_stream).collect::<Vec<_>>().await;
    let res2 = out2.flat_map(into_event_stream).collect::<Vec<_>>().await;

    assert_eq!(vec![event], res1);
    assert_eq!(Vec::<Event>::new(), res2);
}

#[tokio::test]
async fn topology_remove_one_transform() {
    trace_init();

    // Create a simple source/transform/transform/sink topology, wired up in that order:
    let (mut in1, source1) = source();
    let transform1 = transform(" transformed", 0.0);
    let transform2 = transform(" transformed", 0.0);
    let (out1, sink1) = sink(10);

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_transform("t1", &["in1"], transform1);
    config.add_transform("t2", &["t1"], transform2);
    config.add_sink("out1", &["t2"], sink1);

    let (mut topology, _crash) = start_topology(config.build().unwrap(), false).await;

    // Now create an identical topology, but remove one of the transforms:
    let (mut in2, source2) = source();
    let transform2 = transform(" transformed", 0.0);
    let (out2, sink2) = sink(10);

    let mut config = Config::builder();
    config.add_source("in1", source2);
    config.add_transform("t2", &["in1"], transform2);
    config.add_sink("out1", &["t2"], sink2);

    assert!(topology
        .reload_config_and_respawn(config.build().unwrap())
        .await
        .unwrap());

    // Send the same event to both sources:
    let event = Event::from("this");
    let h_out1 = tokio::spawn(out1.flat_map(into_message_stream).collect::<Vec<_>>());
    let h_out2 = tokio::spawn(out2.flat_map(into_message_stream).collect::<Vec<_>>());
    in1.send(event.clone()).await.unwrap();
    in2.send(event.clone()).await.unwrap();

    // Drop the inputs to the two sources, which will ensure they drain all items and stop
    // themselves, and also fully stop the topology:
    drop(in1);
    drop(in2);
    topology.stop().await;

    // We should see that because the source and sink didn't change -- only the one transform being
    // removed -- that the event sent to the first source is the one that makes it through, but that
    // it now goes through the changed transform chain: one transform instead of two.
    let res1 = h_out1.await.unwrap();
    let res2 = h_out2.await.unwrap();
    assert_eq!(vec!["this transformed"], res1);
    assert_eq!(Vec::<String>::new(), res2);
}

#[tokio::test]
async fn topology_swap_source() {
    trace_init();

    // Add source #1 as `in1`, and sink #1 as `out1`, with sink #1 attached to `in1`:
    let (mut in1, source1) = source();
    let (out1, sink1) = sink(10);

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_sink("out1", &["in1"], sink1);

    let (mut topology, _crash) = start_topology(config.build().unwrap(), false).await;

    // Now, create sink #2 and replace `out2` with it, and add source #2 as `in2`, attached to `out1`:
    let (mut in2, source2) = source();
    let (out2, sink2) = sink(10);

    let mut config = Config::builder();
    config.add_source("in2", source2);
    config.add_sink("out1", &["in2"], sink2);

    assert!(topology
        .reload_config_and_respawn(config.build().unwrap())
        .await
        .unwrap());

    // Send an event into both source #1 and source #2:
    let event1 = Event::from("this");
    let event2 = Event::from("that");

    let h_out1 = tokio::spawn(out1.flat_map(into_event_stream).collect::<Vec<_>>());
    let h_out2 = tokio::spawn(out2.flat_map(into_event_stream).collect::<Vec<_>>());
    in1.send(event1.clone()).await.unwrap_err();
    in2.send(event2.clone()).await.unwrap();

    // Drop the inputs to the two sources, which will ensure they drain all items and stop
    // themselves, and also fully stop the topology:
    drop(in1);
    drop(in2);
    topology.stop().await;

    let res1 = h_out1.await.unwrap();
    let res2 = h_out2.await.unwrap();

    // We should see that despite replacing a sink of the same name, sending to source #1 -- which
    // the sink at `out1` was initially connected to -- does not send to either sink #1 or sink #2,
    // as we've removed it from the topology prior to the sends.
    assert_eq!(Vec::<Event>::new(), res1);
    assert_eq!(vec![event2], res2);
}

#[tokio::test]
async fn topology_swap_transform() {
    trace_init();

    // Add source #1 as `in1`, transform #1 as `t1`, and sink #1 as `out1`, with transform #1
    // attached to `in1` and sink #1 attached to `t1`:
    let (mut in1, source1) = source();
    let transform1 = transform(" transformed", 0.0);
    let (out1, sink1) = sink(10);

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_transform("t1", &["in1"], transform1);
    config.add_sink("out1", &["t1"], sink1);

    let (mut topology, _crash) = start_topology(config.build().unwrap(), false).await;

    // Now, create source #2 and replace `in1` with it, add transform #2 as `t1`, attached to `in1`,
    // and add sink #2 as `out1`, attached to `t1`:
    let (mut in2, source2) = source();
    let transform2 = transform(" replaced", 0.0);
    let (out2, sink2) = sink(10);

    let mut config = Config::builder();
    config.add_source("in1", source2);
    config.add_transform("t1", &["in1"], transform2);
    config.add_sink("out1", &["t1"], sink2);

    assert!(topology
        .reload_config_and_respawn(config.build().unwrap())
        .await
        .unwrap());

    // Send an event into both source #1 and source #2:
    let event1 = Event::from("this");
    let event2 = Event::from("that");

    let h_out1 = tokio::spawn(out1.flat_map(into_message_stream).collect::<Vec<_>>());
    let h_out2 = tokio::spawn(out2.flat_map(into_message_stream).collect::<Vec<_>>());
    in1.send(event1.clone()).await.unwrap();
    in2.send(event2.clone()).await.unwrap();

    // Drop the inputs to the two sources, which will ensure they drain all items and stop
    // themselves, and also fully stop the topology:
    drop(in1);
    drop(in2);
    topology.stop().await;

    let res1 = h_out1.await.unwrap();
    let res2 = h_out2.await.unwrap();

    // We should see that since source #1 and #2 were the same, as well as sink #1 and sink #2,
    // despite both being added as `in1`, that source #1 was not rebuilt, so the item sent to source
    // #1 was the item that got transformed, which was emitted via `out1`/`h_out1`/`res1`.
    assert_eq!(vec!["this replaced"], res1);
    assert_eq!(Vec::<String>::new(), res2);
}

#[tokio::test]
async fn topology_swap_sink() {
    trace_init();

    // Add source #1 as `in1`, and sink #1 as `out1`, with sink #1 attached to `in1`:
    let (mut in1, source1) = source();
    let (out1, sink1) = sink_with_data(10, "v1");

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_sink("out1", &["in1"], sink1);

    let (mut topology, _crash) = start_topology(config.build().unwrap(), false).await;

    // Now, create an identical topology except that the sink has changed which will force it to be rebuilt:
    let (mut in2, source2) = source();
    let (out2, sink2) = sink_with_data(10, "v2");

    let mut config = Config::builder();
    config.add_source("in1", source2);
    config.add_sink("out1", &["in1"], sink2);

    assert!(topology
        .reload_config_and_respawn(config.build().unwrap())
        .await
        .unwrap());

    // Send an event into both source #1 and source #2:
    let event1 = Event::from("this");
    let event2 = Event::from("that");

    let h_out1 = tokio::spawn(out1.flat_map(into_event_stream).collect::<Vec<_>>());
    let h_out2 = tokio::spawn(out2.flat_map(into_event_stream).collect::<Vec<_>>());
    in1.send(event1.clone()).await.unwrap();
    in2.send(event2.clone()).await.unwrap();

    // Drop the inputs to the two sources, which will ensure they drain all items and stop
    // themselves, and also fully stop the topology:
    drop(in1);
    drop(in2);
    topology.stop().await;

    let res1 = h_out1.await.unwrap();
    let res2 = h_out2.await.unwrap();

    // We should see that since source #1 and #2 were the same, despite both being added as `in1`,
    // that source #1 was not rebuilt, so the item sent to source #1 was the item that got sent to
    // the new sink, which _was_ rebuilt:
    assert_eq!(Vec::<Event>::new(), res1);
    assert_eq!(vec![event1], res2);
}

#[tokio::test]
async fn topology_rebuild_connected() {
    trace_init();

    let (_in1, source1) = source_with_data("v1");
    let (_out1, sink1) = sink_with_data(10, "v1");

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_sink("out1", &["in1"], sink1);

    let (mut topology, _crash) = start_topology(config.build().unwrap(), false).await;

    let (mut in1, source1) = source_with_data("v2");
    let (out1, sink1) = sink_with_data(10, "v2");

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_sink("out1", &["in1"], sink1);

    assert!(topology
        .reload_config_and_respawn(config.build().unwrap())
        .await
        .unwrap());

    let event1 = Event::from("this");
    let event2 = Event::from("that");
    let h_out1 = tokio::spawn(out1.flat_map(into_event_stream).collect::<Vec<_>>());
    in1.send(event1.clone()).await.unwrap();
    in1.send(event2.clone()).await.unwrap();

    drop(in1);
    topology.stop().await;

    let res = h_out1.await.unwrap();
    assert_eq!(vec![event1, event2], res);
}

#[tokio::test]
async fn topology_rebuild_connected_transform() {
    trace_init();

    let (mut in1, source1) = source_with_data("v1");
    let transform1 = transform(" transformed", 0.0);
    let transform2 = transform(" transformed", 0.0);
    let (out1, sink1) = sink_with_data(10, "v1");

    let mut config = Config::builder();
    config.add_source("in1", source1);
    config.add_transform("t1", &["in1"], transform1);
    config.add_transform("t2", &["t1"], transform2);
    config.add_sink("out1", &["t2"], sink1);

    let (mut topology, _crash) = start_topology(config.build().unwrap(), false).await;

    let (mut in2, source2) = source_with_data("v1"); // not changing
    let transform1 = transform("", 0.0);
    let transform2 = transform("", 0.0);
    let (out2, sink2) = sink_with_data(10, "v2");

    let mut config = Config::builder();
    config.add_source("in1", source2);
    config.add_transform("t1", &["in1"], transform1);
    config.add_transform("t2", &["t1"], transform2);
    config.add_sink("out1", &["t2"], sink2);

    assert!(topology
        .reload_config_and_respawn(config.build().unwrap())
        .await
        .unwrap());

    let event = Event::from("this");
    let h_out1 = tokio::spawn(out1.flat_map(into_event_stream).collect::<Vec<_>>());
    let h_out2 = tokio::spawn(out2.flat_map(into_event_stream).collect::<Vec<_>>());

    in1.send(event.clone()).await.unwrap();
    in2.send(event.clone()).await.unwrap();

    drop(in1);
    drop(in2);
    topology.stop().await;

    let res1 = h_out1.await.unwrap();
    let res2 = h_out2.await.unwrap();
    assert_eq!(Vec::<Event>::new(), res1);
    assert_eq!(vec![event], res2);
}

#[tokio::test]
async fn topology_required_healthcheck_fails_start() {
    let mut config = basic_config_with_sink_failing_healthcheck();
    config.healthchecks.require_healthy = true;
    let diff = framework::config::ConfigDiff::initial(&config);
    let pieces = topology::build_or_log_errors(&config, &diff, HashMap::new())
        .await
        .unwrap();

    assert!(topology::start_validate(config, diff, pieces)
        .await
        .is_none());
}

#[tokio::test]
async fn topology_optional_healthcheck_does_not_fail_start() {
    let config = basic_config_with_sink_failing_healthcheck();
    let diff = framework::config::ConfigDiff::initial(&config);
    let pieces = topology::build_or_log_errors(&config, &diff, HashMap::new())
        .await
        .unwrap();
    assert!(topology::start_validate(config, diff, pieces)
        .await
        .is_some());
}

#[tokio::test]
async fn topology_optional_healthcheck_does_not_fail_reload() {
    let config = basic_config();
    let (mut topology, _crash) = start_topology(config, false).await;
    let config = basic_config_with_sink_failing_healthcheck();
    assert!(topology.reload_config_and_respawn(config).await.unwrap());
}

#[tokio::test]
async fn topology_healthcheck_not_run_on_unchanged_reload() {
    let config = basic_config();

    let (mut topology, _crash) = start_topology(config, false).await;
    let mut config = basic_config_with_sink_failing_healthcheck();
    config.healthchecks.require_healthy = true;
    assert!(topology.reload_config_and_respawn(config).await.unwrap());
}

#[tokio::test]
async fn topology_healthcheck_run_for_changes_on_reload() {
    let mut config = Config::builder();
    // We can't just drop the sender side since that will close the source.
    let (_ch0, src) = source();
    config.add_source("in1", src);
    config.add_sink("out1", &["in1"], sink(10).1);

    let (mut topology, _crash) = start_topology(config.build().unwrap(), false).await;

    let mut config = Config::builder();
    // We can't just drop the sender side since that will close the source.
    let (_ch1, src) = source();
    config.add_source("in1", src);
    config.add_sink("out2", &["in1"], sink_failing_healthcheck(10).1);

    let mut config = config.build().unwrap();
    config.healthchecks.require_healthy = true;
    assert!(!topology.reload_config_and_respawn(config).await.unwrap());
}

#[tokio::test]
async fn topology_disk_buffer_flushes_on_idle() {
    trace_init();

    let tmpdir = tempdir().expect("no tmpdir");
    let event = Event::Log(LogRecord::from("foo"));

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
            max_size: NonZeroU64::new(268435488).unwrap(),
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
