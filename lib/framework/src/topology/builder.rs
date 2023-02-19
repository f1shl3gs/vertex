use std::collections::HashMap;
use std::future::ready;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use buffers::builder::TopologyBuilder;
use buffers::channel::{BufferReceiver, BufferSender};
use buffers::{BufferType, WhenFull};
use event::{EventContainer, Events};
use futures::{FutureExt, StreamExt};
use futures_util::stream::FuturesOrdered;
use measurable::ByteSizeOf;
use metrics::{Attributes, Counter};
use stream_cancel::{StreamExt as StreamCancelExt, Trigger, Tripwire};
use tokio::time::timeout;
use tracing_futures::Instrument;

use super::fanout;
use super::fanout::Fanout;
use super::task::{Task, TaskOutput};
use super::BuiltBuffer;
use crate::config::{
    ComponentKey, Config, ConfigDiff, DataType, ExtensionContext, Output, OutputId, ProxyConfig,
    SinkContext, SourceContext, TransformContext,
};
use crate::metrics::MetricStreamExt;
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownCoordinator;
use crate::{SyncTransform, TaskTransform, Transform, TransformOutputs, TransformOutputsBuf};

pub(crate) const CHUNK_SIZE: usize = 1024;
pub(crate) const TOPOLOGY_BUFFER_SIZE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(128) };

pub struct Pieces {
    pub inputs: HashMap<ComponentKey, (BufferSender<Events>, Vec<OutputId>)>,
    pub outputs: HashMap<ComponentKey, HashMap<Option<String>, fanout::ControlChannel>>,
    pub tasks: HashMap<ComponentKey, Task>,
    pub source_tasks: HashMap<ComponentKey, Task>,
    pub health_checks: HashMap<ComponentKey, Task>,
    pub shutdown_coordinator: ShutdownCoordinator,
    pub detach_triggers: HashMap<ComponentKey, Trigger>,
}

#[derive(Clone, Debug)]
struct TransformNode {
    key: ComponentKey,
    typetag: &'static str,
    inputs: Vec<OutputId>,
    input_type: DataType,
    outputs: Vec<Output>,
    concurrency: bool,
}

fn build_transform(
    transform: Transform,
    node: TransformNode,
    input_rx: BufferReceiver<Events>,
) -> (Task, HashMap<OutputId, fanout::ControlChannel>) {
    match transform {
        Transform::Function(f) => build_sync_transform(Box::new(f), node, input_rx),
        Transform::Synchronous(s) => build_sync_transform(s, node, input_rx),
        Transform::Task(t) => {
            build_task_transform(t, input_rx, node.input_type, node.typetag, &node.key)
        }
    }
}

fn build_sync_transform(
    t: Box<dyn SyncTransform>,
    node: TransformNode,
    input_rx: BufferReceiver<Events>,
) -> (Task, HashMap<OutputId, fanout::ControlChannel>) {
    let (outputs, controls) = TransformOutputs::new(node.outputs);

    let runner = Runner::new(node.key.id(), t, input_rx, node.input_type, outputs);
    let transform = if node.concurrency {
        runner.run_concurrently().boxed()
    } else {
        runner.run_inline().boxed()
    };

    let mut output_controls = HashMap::new();
    for (name, control) in controls {
        let id = name
            .map(|name| OutputId::from((&node.key, name)))
            .unwrap_or_else(|| OutputId::from(&node.key));
        output_controls.insert(id, control);
    }

    let task = Task::new(node.key.clone(), node.typetag, transform);
    (task, output_controls)
}

fn build_task_transform(
    t: Box<dyn TaskTransform>,
    input_rx: BufferReceiver<Events>,
    input_type: DataType,
    typetag: &str,
    key: &ComponentKey,
) -> (Task, HashMap<OutputId, fanout::ControlChannel>) {
    let (mut fanout, control) = Fanout::new();
    let input_rx = crate::utilization::wrap(input_rx);

    let filtered = input_rx.filter(move |events| ready(filter_events_type(events, input_type)));
    let stream = t.transform(Box::pin(filtered));
    let transform = async move {
        fanout.send_stream(stream).await;
        debug!("Finished");
        Ok(TaskOutput::Transform)
    };

    let mut outputs = HashMap::new();
    outputs.insert(OutputId::from(key), control);

    let task = Task::new(key.clone(), typetag, transform);

    (task, outputs)
}

struct Runner {
    transform: Box<dyn SyncTransform>,
    input_rx: Option<BufferReceiver<Events>>,
    input_type: DataType,
    outputs: TransformOutputs,
    timer: crate::utilization::Timer,
    last_report: Instant,

    // metrics
    send_events: Counter,
    send_bytes: Counter,
    received_events: Counter,
    received_bytes: Counter,
}

impl Runner {
    fn new(
        key: &str,
        transform: Box<dyn SyncTransform>,
        input_rx: BufferReceiver<Events>,
        input_type: DataType,
        outputs: TransformOutputs,
    ) -> Self {
        let attrs = Attributes::from([
            ("component", key.to_string().into()),
            ("component_type", "transform".into()),
        ]);
        let send_events = metrics::register_counter(
            "component_sent_events_total",
            "The total number of events emitted by this component.",
        )
        .recorder(attrs.clone());
        let send_bytes = metrics::register_counter(
            "component_sent_event_bytes_total",
            "The total number of event bytes emitted by this component.",
        )
        .recorder(attrs.clone());
        let received_events = metrics::register_counter(
            "component_received_events_total",
            "The number of events accepted by this component either from tagged origins like file and uri, or cumulatively from other origins.",
        ).recorder(attrs.clone());
        let received_bytes = metrics::register_counter(
            "component_received_event_bytes_total",
            "The number of event bytes accepted by this component either from tagged origins like file and uri, or cumulatively from other origins."
        ).recorder(attrs);

        Self {
            transform,
            input_rx: Some(input_rx),
            input_type,
            outputs,
            timer: crate::utilization::Timer::new(),
            last_report: Instant::now(),
            // metrics
            send_events,
            send_bytes,
            received_events,
            received_bytes,
        }
    }

    fn on_events_received(&mut self, events: &Events) {
        let stopped = self.timer.stop_wait();
        if stopped.duration_since(self.last_report).as_secs() >= 5 {
            self.timer.report();
            self.last_report = stopped;
        }

        let count = events.len();
        let byte_size = events.size_of();

        trace!(message = "Events received", count, byte_size);

        self.received_events.inc(count as u64);
        self.received_bytes.inc(byte_size as u64);
    }

    async fn send_outputs(&mut self, outputs_buf: &mut TransformOutputsBuf) {
        // TODO: account for named outputs separately?
        let count = outputs_buf.len();
        // TODO: do we only want allocated_bytes for events themselves?
        let byte_size = outputs_buf.size_of();

        self.timer.start_wait();
        self.outputs.send(outputs_buf).await;

        trace!(
            message = "Events sent",
            count = %count,
            byte_size = %byte_size
        );

        self.send_events.inc(count as u64);
        self.send_bytes.inc(byte_size as u64);
    }

    async fn run_inline(mut self) -> Result<TaskOutput, ()> {
        // 128 is an arbitrary, smallish constant
        const INLINE_BATCH_SIZE: usize = 128;

        let mut outputs_buf = self.outputs.new_buf_with_capacity(INLINE_BATCH_SIZE);
        let mut input_rx = self
            .input_rx
            .take()
            .expect("can't run runner twice")
            .filter(move |event| ready(filter_events_type(event, self.input_type)));

        self.timer.start_wait();
        while let Some(events) = input_rx.next().await {
            self.on_events_received(&events);
            self.transform.transform(events, &mut outputs_buf);
            self.send_outputs(&mut outputs_buf).await;
        }

        debug!(message = "Finished");

        Ok(TaskOutput::Transform)
    }

    async fn run_concurrently(mut self) -> Result<TaskOutput, ()> {
        // TODO: Retrieving tokio runtime worker num is a better solution.
        //
        // There is no API for retrieve Tokio's runtime worker num. `RuntimeMetrics` can do that,
        // but it is not stable yet.
        let concurrency_limit = crate::num_workers();
        let mut input_rx = self
            .input_rx
            .take()
            .expect("can't run runer twice")
            .filter(move |event| ready(filter_events_type(event, self.input_type)));
        let mut in_flight = FuturesOrdered::new();
        let mut shutting_down = false;

        self.timer.start_wait();
        loop {
            tokio::select! {
                biased;

                result = in_flight.next(), if !in_flight.is_empty() => {
                    match result {
                        Some(Ok(outputs_buf)) => {
                            let mut outputs_buf: TransformOutputsBuf = outputs_buf;
                            self.send_outputs(&mut outputs_buf).await;
                        }

                        _ => unreachable!("join error or bad poll"),
                    }
                }

                input_events = input_rx.next(), if in_flight.len() < concurrency_limit && !shutting_down => {
                    match input_events {
                        Some(events) => {
                            self.on_events_received(&events);

                            let mut t = self.transform.clone();
                            let mut outputs_buf = self.outputs.new_buf_with_capacity(events.len());
                            let task = tokio::spawn(async move {
                                t.transform(events, &mut outputs_buf);
                                outputs_buf
                            });
                            in_flight.push_back(task);
                        },

                        None => {
                            shutting_down = true;
                            continue
                        }
                    }
                }

                else => {
                    if shutting_down {
                        break
                    }
                }
            }
        }

        debug!(message = "Finished");
        Ok(TaskOutput::Transform)
    }
}

/// Builds only the new pieces, and doesn't check their topology.
pub async fn build_pieces(
    config: &Config,
    diff: &ConfigDiff,
    mut buffers: HashMap<ComponentKey, BuiltBuffer>,
) -> Result<Pieces, Vec<String>> {
    let mut inputs = HashMap::new();
    let mut outputs = HashMap::new();
    let mut detach_triggers = HashMap::new();
    let mut tasks = HashMap::new();
    let mut source_tasks = HashMap::new();
    let mut shutdown_coordinator = ShutdownCoordinator::default();
    let mut health_checks = HashMap::new();
    let mut errors = vec![];

    // Build extensions
    for (key, extension) in config
        .extensions
        .iter()
        .filter(|(name, _)| diff.extensions.contains_new(name))
    {
        let typetag = extension.component_name();
        let (shutdown_signal, force_shutdown_tripwire) =
            shutdown_coordinator.register_extension(key);
        let cx = ExtensionContext {
            name: key.to_string(),
            global: config.global.clone(),
            shutdown: shutdown_signal,
        };

        let ext = match extension.build(cx).await {
            Ok(ext) => ext,
            Err(err) => {
                errors.push(format!("Extension {}: {}", key, err));

                continue;
            }
        };

        // The force_shutdown_tripwire is a Future that when it resolves means that
        // this source has failed to shut down gracefully within its allotted time
        // window and instead should be forcibly shut down. We accomplish this
        // by select()-ing on the server Task with the force_shutdown_tripwire.
        // That means that if the force_shutdown_tripwire resolves while the
        // server Task is still running the Task will simply be dropped on the floor.
        let server = async {
            let result = tokio::select! {
                biased;

                _ = force_shutdown_tripwire => {
                    Ok(())
                },
                result = ext => result,
            };

            match result {
                Ok(()) => {
                    debug!(message = "Finished.");
                    Ok(TaskOutput::Source)
                }
                Err(()) => Err(()),
            }
        };

        let task = Task::new(key.clone(), typetag, server);
        tasks.insert(key.clone(), task);
    }

    // Build sources
    for (key, source) in config
        .sources
        .iter()
        .filter(|(name, _)| diff.sources.contains_new(name))
    {
        debug!(message = "Building new source", component = %key);

        let typetag = source.inner.component_name();
        let source_outputs = source.inner.outputs();

        let span = error_span!(
            "source",
            component_kind = "source",
            component_id = %key.id(),
            component_type = %source.inner.component_name(),
        );

        let mut builder = Pipeline::builder().with_buffer(CHUNK_SIZE * crate::num_workers());
        let mut pumps = Vec::new();
        let mut controls = HashMap::new();

        for output in source_outputs {
            let mut rx = builder.add_output(key.id(), output.clone());
            let (mut fanout, control) = Fanout::new();
            let pump = async move {
                debug!(message = "Source pump starting");
                while let Some(events) = rx.next().await {
                    fanout.send(events).await;
                }
                debug!(message = "Source pump finished");
                Ok(TaskOutput::Source)
            };

            pumps.push(pump.instrument(span.clone()));
            controls.insert(
                OutputId {
                    component: key.clone(),
                    port: output.port,
                },
                control,
            );
        }

        let pump = async move {
            let mut handles = Vec::new();
            for pump in pumps {
                handles.push(tokio::spawn(pump));
            }

            for handle in handles {
                handle.await.expect("join error")?;
            }

            Ok(TaskOutput::Source)
        };
        let pump = Task::new(key.clone(), typetag, pump);
        let pipeline = builder.build();

        let (shutdown_signal, force_shutdown_tripwire) = shutdown_coordinator.register_source(key);
        let cx = SourceContext {
            key: key.clone(),
            output: pipeline,
            shutdown: shutdown_signal,
            globals: config.global.clone(),
            proxy: ProxyConfig::merge_with_env(&config.global.proxy, &source.proxy),
            acknowledgements: source.acknowledgements,
        };
        let server = match source.inner.build(cx).await {
            Ok(server) => server,
            Err(err) => {
                errors.push(format!("Source \"{}\": {}", key, err));
                continue;
            }
        };

        // The force_shutdown_tripwire is a Future that when it resolves means
        // that this source has failed to shut down gracefully within its
        // allotted time window and instead should forcibly shut down. We
        // accomplish this by select()-ing on the server Task with the
        // force_shutdown_tripwire. That means that if the force_shutdown_tripwire
        // resolves while the server Task is still running the Task will simply
        // be dropped on the floor.
        let server = async {
            let result = tokio::select! {
                biased;

                _ = force_shutdown_tripwire => {
                    Ok(())
                },
                result = server => result
            };

            match result {
                Ok(()) => {
                    debug!(message = "Finished");
                    Ok(TaskOutput::Source)
                }
                Err(()) => Err(()),
            }
        };

        let server = Task::new(key.clone(), typetag, server);
        outputs.extend(controls);
        tasks.insert(key.clone(), pump);
        source_tasks.insert(key.clone(), server);
    }

    // Build transforms
    for (key, transform) in config
        .transforms
        .iter()
        .filter(|(name, _)| diff.transforms.contains_new(name))
    {
        let ctx = TransformContext {
            key: Some(key.clone()),
            globals: config.global.clone(),
        };

        let node = TransformNode {
            key: key.clone(),
            typetag: transform.inner.component_name(),
            inputs: transform.inputs.clone(),
            input_type: transform.inner.input_type(),
            outputs: transform.inner.outputs(),
            concurrency: transform.inner.enable_concurrency(),
        };

        let transform = match transform.inner.build(&ctx).await {
            Ok(trans) => trans,
            Err(err) => {
                errors.push(format!("Transform \"{}\": {}", key, err));
                continue;
            }
        };

        let (input_tx, input_rx) =
            TopologyBuilder::standalone_memory(TOPOLOGY_BUFFER_SIZE, WhenFull::Block).await;
        inputs.insert(key.clone(), (input_tx, node.inputs.clone()));

        let (transform_task, transform_outputs) = build_transform(transform, node, input_rx);
        outputs.extend(transform_outputs);
        tasks.insert(key.clone(), transform_task);
    }

    // Build sinks
    for (name, sink) in config
        .sinks
        .iter()
        .filter(|(name, _)| diff.sinks.contains_new(name))
    {
        let sink_inputs = &sink.inputs;
        let health_check = sink.health_check();
        let enable_health_check = health_check && config.healthchecks.enabled;
        let typetag = sink.inner.component_name();
        let input_type = sink.inner.input_type();

        let (tx, rx, acker) = if let Some(buffer) = buffers.remove(name) {
            buffer
        } else {
            let buffer_type = match sink.buffer.stages().first().expect("cant ever be empty") {
                BufferType::Memory { .. } => "memory",
                BufferType::Disk { .. } => "disk",
            };

            let buffer_span = error_span!(
                "sink",
                component_kind = "sink",
                component_id = %name,
                component_type = typetag,
                buffer_type = buffer_type,
            );
            let buffer = sink
                .buffer
                .build(
                    config.global.data_dir.clone(),
                    name.to_string(),
                    buffer_span,
                )
                .await;

            match buffer {
                Ok((tx, rx, acker)) => (tx, Arc::new(Mutex::new(Some(rx))), acker),
                Err(err) => {
                    // TODO: handle BufferBuildError properly
                    errors.push(format!("Sink \"{}\": {:?}", name, err));
                    continue;
                }
            }
        };

        let cx = SinkContext {
            acker: acker.clone(),
            health_check,
            globals: config.global.clone(),
            proxy: ProxyConfig::merge_with_env(&config.global.proxy, sink.proxy()),
        };

        let (sink, healthcheck) = match sink.inner.build(cx).await {
            Ok(built) => built,
            Err(err) => {
                errors.push(format!("Sink \"{}\": {}", name, err));
                continue;
            }
        };

        let (trigger, tripwire) = Tripwire::new();
        let component = name.id().to_string();
        let attrs = Attributes::from([
            ("component", component.into()),
            ("component_type", "sink".into()),
        ]);
        let sink = async move {
            // Why is this Arc<Mutex<Option<_>>> needed you may ask.
            // In case when this function build_pieces errors this
            // future won't be run so this rx won't be taken which
            // will enable us to reuse rx to rebuild old configuration
            // by passing this Arc<Mutex<Option<_>>> yet again.
            let rx = rx
                .lock()
                .unwrap()
                .take()
                .expect("Task started but input has been taken");

            let mut rx = crate::utilization::wrap(rx);

            sink.run(
                rx.by_ref()
                    .filter(|events| ready(filter_events_type(events, input_type)))
                    .take_until_if(tripwire)
                    .metric_record(attrs),
            )
            .await
            .map(|_| {
                debug!(message = "Finished");
                TaskOutput::Sink(rx, acker)
            })
        };

        let task = Task::new(name.clone(), typetag, sink);
        let component_key = name.clone();
        let healthcheck_task = async move {
            if enable_health_check {
                let duration = Duration::from_secs(10);
                timeout(duration, healthcheck)
                    .map(|result| match result {
                        Ok(Ok(_)) => {
                            info!("Healthcheck: Passed");
                            Ok(TaskOutput::HealthCheck)
                        }

                        Ok(Err(err)) => {
                            error!(
                                message = "Healthcheck: Failed",
                                %err,
                                component_kind = "sink",
                                component_id = %component_key,
                            );

                            Err(())
                        }

                        Err(_) => {
                            error!(
                                message = "Healthcheck: timeout",
                                component_kind = "sink",
                                component_id = %component_key,
                            );

                            Err(())
                        }
                    })
                    .await
            } else {
                info!("Healthcheck: Disabled");
                Ok(TaskOutput::HealthCheck)
            }
        };

        let healthcheck_task = Task::new(name.clone(), typetag, healthcheck_task);
        inputs.insert(name.clone(), (tx, sink_inputs.clone()));
        health_checks.insert(name.clone(), healthcheck_task);
        tasks.insert(name.clone(), task);
        detach_triggers.insert(name.clone(), trigger);
    }

    let mut finalized_outputs = HashMap::new();
    for (id, output) in outputs {
        let entry = finalized_outputs
            .entry(id.component)
            .or_insert_with(HashMap::new);
        entry.insert(id.port, output);
    }

    if errors.is_empty() {
        Ok(Pieces {
            inputs,
            outputs: finalized_outputs,
            tasks,
            source_tasks,
            health_checks,
            shutdown_coordinator,
            detach_triggers,
        })
    } else {
        Err(errors)
    }
}

fn filter_events_type(events: &Events, data_type: DataType) -> bool {
    if data_type == DataType::All {
        return true;
    }

    match events {
        Events::Logs(_) => data_type.contains(DataType::Log),
        Events::Metrics(_) => data_type.contains(DataType::Metric),
        Events::Traces(_) => data_type.contains(DataType::Trace),
    }
}
