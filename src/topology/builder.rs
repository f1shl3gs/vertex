use std::time::{Duration, Instant};
use std::{
    collections::HashMap,
    future::ready,
    sync::{Arc, Mutex},
};

use crate::{
    config::{Config, ConfigDiff, DataType, SinkContext, SourceContext},
    pipeline::Pipeline,
    shutdown::ShutdownCoordinator,
    topology::fanout::Fanout,
    topology::task::{Task, TaskOutput},
    transforms::Transform,
};

use super::BuiltBuffer;
use crate::config::{
    ComponentKey, ExtensionContext, Output, OutputId, ProxyConfig, TransformContext,
};
use crate::topology::fanout;
use crate::transforms::{SyncTransform, TaskTransform, TransformOutputs, TransformOutputsBuf};
use buffers::builder::TopologyBuilder;
use buffers::channel::{BufferReceiver, BufferSender};
use buffers::{BufferType, WhenFull};
use event::Event;
use futures::{FutureExt, SinkExt, StreamExt, TryFutureExt};
use futures_util::stream::FuturesOrdered;
use shared::ByteSizeOf;
use stream_cancel::{StreamExt as StreamCancelExt, Trigger, Tripwire};
use tokio::time::timeout;

const DEFAULT_BUFFER_SIZE: usize = 1024;

pub const SOURCE_SENDER_BUFFER_SIZE: usize = 1024;

// TODO: this should be configured by user
static TRANSFORM_CONCURRENCY_LIMIT: usize = 8;

pub struct Pieces {
    pub inputs: HashMap<ComponentKey, (BufferSender<Event>, Vec<OutputId>)>,
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
    input_rx: BufferReceiver<Event>,
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
    input_rx: BufferReceiver<Event>,
) -> (Task, HashMap<OutputId, fanout::ControlChannel>) {
    let (outputs, controls) = TransformOutputs::new(node.outputs);

    let runner = Runner::new(t, input_rx, node.input_type, outputs);
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
    input_rx: BufferReceiver<Event>,
    input_type: DataType,
    typetag: &str,
    key: &ComponentKey,
) -> (Task, HashMap<OutputId, fanout::ControlChannel>) {
    let (output, control) = Fanout::new();
    let input_rx = crate::utilization::wrap(input_rx);

    let filtered = input_rx
        .filter(move |event| ready(filter_event_type(event, input_type)))
        .inspect(|event| {
            let count = 1;
            let byte_size = event.size_of();

            trace!(message = "Events received", count, byte_size);

            counter!("component_received_events_total", 1);
            counter!("component_received_event_bytes_total", byte_size as u64);
        });
    let transform = t
        .transform(Box::pin(filtered))
        .map(Ok)
        .forward(output.with(|event: Event| async {
            let byte_size = event.size_of();

            trace!(
                message = "Events sent",
                count = 1,
                byte_size = %byte_size
            );

            counter!("component_sent_events_total", 1);
            counter!("component_sent_event_bytes_total", event.size_of() as u64);

            Ok(event)
        }))
        .boxed()
        .map_ok(|_| {
            debug!(message = "Finished");
            TaskOutput::Transform
        });

    let mut outputs = HashMap::new();
    outputs.insert(OutputId::from(key), control);

    let task = Task::new(key.clone(), typetag, transform);

    (task, outputs)
}

struct Runner {
    transform: Box<dyn SyncTransform>,
    input_rx: Option<BufferReceiver<Event>>,
    input_type: DataType,
    outputs: TransformOutputs,
    timer: crate::utilization::Timer,
    last_report: Instant,
}

impl Runner {
    fn new(
        transform: Box<dyn SyncTransform>,
        input_rx: BufferReceiver<Event>,
        input_type: DataType,
        outputs: TransformOutputs,
    ) -> Self {
        Self {
            transform,
            input_rx: Some(input_rx),
            input_type,
            outputs,
            timer: crate::utilization::Timer::new(),
            last_report: Instant::now(),
        }
    }

    fn on_events_received(&mut self, events: &[Event]) {
        let stopped = self.timer.stop_wait();
        if stopped.duration_since(self.last_report).as_secs() >= 5 {
            self.timer.report();
            self.last_report = stopped;
        }

        let count = events.len();
        let byte_size = events.size_of();

        trace!(message = "Events received", count, byte_size);

        counter!("component_received_events_total", count as u64);
        counter!("component_received_event_bytes_total", byte_size as u64);
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

        counter!("component_sent_events_total", count as u64);
        counter!("component_sent_event_bytes_total", byte_size as u64);
    }

    async fn run_inline(mut self) -> Result<TaskOutput, ()> {
        // 128 is an arbitrary, smallish constant
        const INLINE_BATCH_SIZE: usize = 128;

        let mut outputs_buf = self.outputs.new_buf_with_capacity(INLINE_BATCH_SIZE);
        let mut input_rx = self
            .input_rx
            .take()
            .expect("can't run runner twice")
            .filter(move |event| ready(filter_event_type(event, self.input_type)))
            .ready_chunks(INLINE_BATCH_SIZE);

        self.timer.start_wait();
        while let Some(events) = input_rx.next().await {
            self.on_events_received(&events);

            for event in events {
                self.transform.transform(event, &mut outputs_buf);
            }

            self.send_outputs(&mut outputs_buf).await;
        }

        debug!(message = "Finished");

        Ok(TaskOutput::Transform)
    }

    async fn run_concurrently(mut self) -> Result<TaskOutput, ()> {
        // 1024 is an arbitrary, medium-ish constant, larger than the inline runner's batch
        // size to try to balance out the increased overhead of spawning tasks
        const CONCURRENT_BATCH_SIZE: usize = 1024;

        let mut input_rx = self
            .input_rx
            .take()
            .expect("can't run runer twice")
            .filter(move |event| ready(filter_event_type(event, self.input_type)))
            .ready_chunks(CONCURRENT_BATCH_SIZE);

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

                input_events = input_rx.next(), if in_flight.len() < TRANSFORM_CONCURRENCY_LIMIT && !shutting_down => {
                    match input_events {
                        Some(events) => {
                            self.on_events_received(&events);

                            let mut t = self.transform.clone();
                            let mut outputs_buf = self.outputs.new_buf_with_capacity(events.len());
                            let task = tokio::spawn(async move {
                                for event in events {
                                    t.transform(event, &mut outputs_buf);
                                }

                                outputs_buf
                            });
                            in_flight.push(task);
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
    let health_checks_enabled = config.health_checks.enabled;
    let mut errors = vec![];

    // Build extensions
    for (key, extension) in config
        .extensions
        .iter()
        .filter(|(name, _)| diff.extensions.contains_new(name))
    {
        let typetag = extension.extension_type();
        let (shutdown_signal, force_shutdown_tripwire) =
            shutdown_coordinator.register_extension(key);
        let ctx = ExtensionContext {
            name: key.to_string(),
            global: config.global.clone(),
            shutdown: shutdown_signal,
        };

        let ext = match extension.build(ctx).await {
            Ok(ext) => ext,
            Err(err) => {
                errors.push(format!("Extension {}: {}", key, err));

                continue;
            }
        };

        let task = Task::new(key.clone(), typetag, async {
            match futures::future::try_select(ext, force_shutdown_tripwire.unit_error().boxed())
                .await
            {
                Ok(_) => Ok(TaskOutput::Source),
                Err(_) => Err(()),
            }
        });

        let task = Task::new(key.clone(), typetag, task);
        tasks.insert(key.clone(), task);
    }

    // Build sources
    for (key, source) in config
        .sources
        .iter()
        .filter(|(name, _)| diff.sources.contains_new(name))
    {
        let typetag = source.inner.source_type();
        let source_outputs = source.inner.outputs();
        let mut builder = Pipeline::builder().with_buffer(DEFAULT_BUFFER_SIZE);
        let mut pumps = Vec::new();
        let mut controls = HashMap::new();
        for output in source_outputs {
            let mut rx = builder.add_output(output.clone());
            let (mut fanout, control) = Fanout::new();
            let pump = async move {
                while let Some(event) = rx.next().await {
                    fanout.feed(event).await?
                }

                fanout.flush().await?;
                Ok(TaskOutput::Source)
            };

            pumps.push(pump);
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
        let ctx = SourceContext {
            key: key.clone(),
            output: pipeline,
            shutdown: shutdown_signal,
            globals: config.global.clone(),
            proxy: ProxyConfig::merge_with_env(&config.global.proxy, &source.proxy),
        };
        let server = match source.inner.build(ctx).await {
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
                Err(err) => Err(()),
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
            typetag: transform.inner.transform_type(),
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

        let (input_tx, input_rx) = TopologyBuilder::memory(128, WhenFull::Block).await;
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
        let enable_health_check = health_check && config.health_checks.enabled;
        let typetag = sink.inner.sink_type();
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

        let ctx = SinkContext {
            acker: acker.clone(),
            health_check,
            globals: config.global.clone(),
            proxy: ProxyConfig::merge_with_env(&config.global.proxy, sink.proxy()),
        };

        let (sink, healthcheck) = match sink.inner.build(ctx).await {
            Ok(built) => built,
            Err(err) => {
                errors.push(format!("Sink \"{}\": {}", name, err));
                continue;
            }
        };

        let (trigger, tripwire) = Tripwire::new();
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
                    .filter(|event| ready(filter_event_type(event, input_type)))
                    .inspect(|event| {
                        let count = 1;
                        let byte_size = event.size_of();

                        trace!(message = "Events received", count, byte_size);

                        counter!("component_received_events_total", count);
                        counter!("component_received_event_bytes_total", byte_size as u64);
                    })
                    .take_until_if(tripwire),
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
            .or_insert(HashMap::new());
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

fn filter_event_type(event: &Event, data_type: DataType) -> bool {
    match data_type {
        DataType::Any => true,
        DataType::Log => matches!(event, Event::Log(_)),
        DataType::Metric => matches!(event, Event::Metric(_)),
        // DataType::Trace => matches!(event, Event::)
        _ => panic!("unknown event type"),
    }
}
