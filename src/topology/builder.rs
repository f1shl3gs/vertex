use std::{
    collections::HashMap,
    future::ready,
    sync::{Arc, Mutex},
};

use crate::{
    transforms::Transform,
    topology::fanout::Fanout,
    config::{
        Config, SourceContext, ConfigDiff,
        SinkContext, DataType,
    }, shutdown::ShutdownCoordinator, pipeline::Pipeline, topology::task::{
        TaskOutput,
        Task,
    }, buffers,
};

use stream_cancel::{Trigger, Tripwire};
use futures::{StreamExt, SinkExt, TryFutureExt, FutureExt};
use event::Event;
use crate::config::ExtensionContext;
use crate::topology::fanout::ControlChannel;
use super::{BuiltBuffer};


const DEFAULT_CHANNEL_BUFFER_SIZE: usize = 1024;

pub struct Pieces {
    pub inputs: HashMap<String, (buffers::BufferInputCloner<Event>, Vec<String>)>,
    pub outputs: HashMap<String, ControlChannel>,
    pub tasks: HashMap<String, Task>,
    pub source_tasks: HashMap<String, Task>,
    pub health_checks: HashMap<String, Task>,
    pub shutdown_coordinator: ShutdownCoordinator,
    pub detach_triggers: HashMap<String, Trigger>,
}

/// Builds only the new pieces, and doesn't check their topology.
pub async fn build_pieces(
    config: &Config,
    diff: &ConfigDiff,
    mut buffers: HashMap<String, BuiltBuffer>,
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
    for (name, extension) in config
        .extensions
        .iter().
        filter(|(name, _)| diff.extensions.contains_new(name))
    {
        let typetag = extension.extension_type();
        let (shutdown_signal, force_shutdown_tripwire) = shutdown_coordinator.register_extension(name);
        let ctx = ExtensionContext {
            name: name.to_string(),
            global: config.global.clone(),
            shutdown: shutdown_signal,
        };

        let ext = match extension.build(ctx).await {
            Ok(ext) => ext,
            Err(err) => {
                errors.push(
                    format!("Extension {}: {}", name, err)
                );

                continue;
            }
        };

        let task = Task::new(name, typetag, async {
            match futures::future::try_select(
                ext,
                force_shutdown_tripwire.unit_error().boxed(),
            ).await {
                Ok(_) => {
                    Ok(TaskOutput::Source)
                }
                Err(_) => Err(())
            }
        });

        let task = Task::new(name, typetag, task);
        tasks.insert(name.clone(), task);
    }

    // Build sources
    for (name, source) in config
        .sources
        .iter()
        .filter(|(name, _)| diff.sources.contains_new(name))
    {
        let (tx, rx) = futures::channel::mpsc::channel::<Event>(DEFAULT_CHANNEL_BUFFER_SIZE);
        let pipeline = Pipeline::from_sender(tx, vec![]);
        let typetag = source.source_type();
        let (shutdown_signal, force_shutdown_tripwire) = shutdown_coordinator.register_source(name);
        let ctx = SourceContext {
            name: name.clone(),
            out: pipeline,
            shutdown: shutdown_signal,
            global: config.global.clone(),
        };

        let src = match source.build(ctx).await {
            Ok(s) => s,
            Err(err) => {
                errors.push(
                    format!("Source {}: {}", name, err)
                );
                continue;
            }
        };

        let (output, control) = Fanout::new();
        let pump = rx
            .map(Ok)
            .forward(output)
            .map_ok(|_| TaskOutput::Source);
        let pump = Task::new(name, typetag, pump);

        // The force_shutdown_tripwire is a Future that when it resolves means
        // that this source has failed to shut down gracefully within its
        // allotted time window and instead should forcibly shut down. We
        // accomplish this by select()-ing on the server Task with the
        // force_shutdown_tripwire. That means that if the force_shutdown_tripwire
        // resolves while the server Task is still running the Task will simply
        // be dropped on the floor.
        let task = async {
            match futures::future::try_select(src, force_shutdown_tripwire.unit_error().boxed()).await {
                Ok(_) => {
                    Ok(TaskOutput::Source)
                }
                Err(_) => Err(())
            }
        };
        let task = Task::new(name, typetag, task);

        outputs.insert(name.clone(), control);
        tasks.insert(name.clone(), pump);
        source_tasks.insert(name.clone(), task);
    }

    // Build transforms
    for (name, transform) in config
        .transforms
        .iter()
        .filter(|(name, _)| diff.transforms.contains_new(name))
    {
        let trans_inputs = &transform.inputs;
        let typetag = transform.inner.transform_type();
        let input_type = transform.inner.input_type();
        let transform = match transform.inner.build(&config.global).await {
            Ok(t) => t,
            Err(err) => {
                errors.push(
                    format!("Transform {}, {}", name, err)
                );
                continue;
            }
        };


        let (input_tx, input_rx, _) = crate::buffers::BufferConfig::default()
            .build(&config.global.data_dir, "")
            .unwrap();
        let (output, control) = Fanout::new();
        let transform = match transform {
            Transform::Function(mut t) => input_rx
                .filter(move |event| ready(filter_event_type(event, input_type)))
                // .inspect()
                .flat_map(move |v| {
                    let mut buf = Vec::with_capacity(1);
                    t.transform(&mut buf, v);
                    futures::stream::iter(buf.into_iter()).map(Ok)
                })
                .forward(output)
                .boxed(),
            Transform::Task(t) => {
                let filtered = input_rx
                    .filter(move |event| ready(filter_event_type(event, input_type)));
                // .inspect()

                t.transform(Box::pin(filtered))
                    .map(Ok)
                    .forward(output.with(|event| async {
                        Ok(event)
                    }))
                    .boxed()
            }
        }
            .map_ok(|_| {
                TaskOutput::Transform
            });

        let task = Task::new(name, typetag, transform);
        inputs.insert(name.clone(), (input_tx, trans_inputs.clone()));
        outputs.insert(name.clone(), control);
        tasks.insert(name.clone(), task);
    }

    // Build sinks
    for (name, sink) in config
        .sinks
        .iter()
        .filter(|(name, _)| diff.sinks.contains_new(name))
    {
        let sink_inputs = &sink.inputs;
        // let healthcheck = sink.healthcheck();
        let typetag = sink.inner.sink_type();
        let input_type = sink.inner.input_type();

        let (tx, rx, acker) = if let Some(buf) = buffers.remove(name) {
            buf
        } else {
            let buf = sink.buffer.build(&config.global.data_dir, name);
            match buf {
                Ok((tx, rx, acker)) => (tx, Arc::new(Mutex::new(Some(rx.into()))), acker),
                Err(err) => {
                    errors.push(format!("Sink {}: {}", name, err));
                    continue;
                }
            }
        };

        let ctx = SinkContext {
            acker: acker.clone(),
            globals: config.global.clone(),
        };

        let (sink, health_check) = match sink.inner.build(ctx).await {
            Ok(s) => s,
            Err(err) => {
                errors.push(format!("Sink {}: {}", name, err));
                continue;
            }
        };

        let (trigger, tripwire) = Tripwire::new();
        let sink = async move {
            // Why is this Arc<Mutex<Option<_>>> needed you may ask.
            // In case when this function build_pieces errors
            // this future won't be run so this rx won't be taken
            // which will enable us to reuse rx to rebuild old configuration
            // by passing this Arc<Mutex<Option<_>>> yet again.
            let rx = rx
                .lock()
                .unwrap()
                .take()
                .expect("Task started but input has been taken");

            let mut rx = Box::pin(rx);
            sink.run(
                rx.by_ref()
                    .filter(|event| ready(filter_event_type(event, input_type)))
                    // .inspect()
                    .take_until(tripwire),
            )
                .await
                .map(|_| {
                    TaskOutput::Sink(rx, acker)
                })
        };

        let task = Task::new(name, typetag, sink);
        let id = name.clone();
        let health_check_task = async move {
            if health_checks_enabled {
                let duration = std::time::Duration::from_secs(10);
                tokio::time::timeout(duration, health_check)
                    .map(|result| match result {
                        Ok(Ok(_)) => {
                            info!(
                                message = "Health check passed",
                                kind = "sink",
                                typetag,
                                ?id,
                            );
                            Ok(TaskOutput::HealthCheck)
                        }

                        Ok(Err(err)) => {
                            error!(
                                message = "Health check failed",
                                %err,
                                kind = "sink",
                                typetag,
                                ?id,
                            );

                            Err(())
                        }

                        Err(_) => {
                            error!(
                                message = "Health check timeout",
                                kind = "sink",
                                typetag,
                                ?id,
                            );

                            Err(())
                        }
                    }).await
            } else {
                info!("Health check disabled");
                Ok(TaskOutput::HealthCheck)
            }
        };
        let health_check_task = Task::new(name.clone(), typetag, health_check_task);

        inputs.insert(name.clone(), (tx, sink_inputs.clone()));
        tasks.insert(name.clone(), task);
        health_checks.insert(name.clone(), health_check_task);
        detach_triggers.insert(name.clone(), trigger);
    }

    if errors.is_empty() {
        Ok(Pieces {
            tasks,
            source_tasks,
            shutdown_coordinator,
            health_checks,
            inputs,
            outputs,
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
        _ => panic!("unknown event type")
    }
}
