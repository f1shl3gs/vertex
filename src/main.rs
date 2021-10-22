extern crate vertex;

#[cfg(feature = "allocator-mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "allocator-jemalloc")]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

use tokio::runtime;
use tokio::time::Duration;

extern crate chrono;
extern crate chrono_tz;

use clap::{AppSettings, Clap};

use vertex::{
    signal::{self, SignalTo},
    config::{self, ConfigPath, FormatHint},
    topology,
};
use std::collections::HashMap;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use tracing::{info, warn, error, dispatcher::{set_global_default}, Dispatch};
use tracing_log::LogTracer;
use tracing_subscriber::layer::SubscriberExt;

#[derive(Clap, Debug)]
#[clap(version = "0.1.0")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(short, long, default_value = "/etc/vertex/vertex.conf")]
    pub config: String,

    // todo sub commands
}

fn init(color: bool, json: bool, levels: &str) {
    // An escape hatch to disable injecting a metrics layer into tracing.
    // May be used for performance reasons. This is a hidden and undocumented functionality.
    let metrics_layer_enabled = !matches!(
        std::env::var("DISABLE_INTERNAL_METRICS_TRACING_INTEGRATION"),
        Ok(x) if x == "true"
    );

    #[cfg(feature = "tokio-console")]
        let subscriber = {
        let (tasks_layer, tasks_server) = console_subscriber::TasksLayer::new();
        tokio::spawn(tasks_server.serve());

        tracing_subscriber::registry::Registry::default()
            .with(tasks_layer)
            .with(tracing_subscriber::filter::EnvFilter::from(levels))
    };

    #[cfg(not(feature = "tokio-console"))]
        let subscriber = tracing_subscriber::registry::Registry::default()
        .with(tracing_subscriber::filter::EnvFilter::from(levels));

    // dev note: we attempted to refactor to reduce duplication but it was starting to seem like
    // the refactored code would be introducting more complexity than it was worth to remove this
    // bit of duplication as we started to create a generic struct to wrap the formatters that
    // also implement `Layer`
    let dispatch = if json {
        #[cfg(not(test))]
            let formatter = tracing_subscriber::fmt::Layer::default()
            .json()
            .flatten_event(true);

        #[cfg(test)]
            let formatter = tracing_subscriber::fmt::Layer::default()
            .json()
            .flatten_event(true)
            .with_test_writer(); // ensures output is captured

        // TODO: rate limit
        let s = subscriber.with(formatter);
        Dispatch::new(s)
    } else {
        #[cfg(not(test))]
            let formatter = tracing_subscriber::fmt::Layer::default()
            .with_ansi(color)
            .with_writer(std::io::stderr);

        #[cfg(test)]
            let formatter = tracing_subscriber::fmt::Layer::default()
            .with_ansi(color)
            .with_test_writer(); // ensures output is captured

        // TODO: rate limit

        let s = subscriber.with(formatter);
        Dispatch::new(s)
    };

    let _ = LogTracer::init().expect("init log tracer failed");
    let _ = set_global_default(dispatch);
}

fn main() {
    let opts: Opts = Opts::parse();

    let rt = runtime::Builder::new_multi_thread()
        // .worker_threads(4)
        .thread_name("vertex-worker")
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    info!(
        message = "start vertex",
        config = ?opts.config
    );

    rt.block_on(async move {
        #[cfg(test)]
            init(true, false, "debug");
        #[cfg(not(test))]
            init(true, false, "info");

        let (mut signal_handler, mut signal_rx) = signal::SignalHandler::new();
        signal_handler.forever(signal::os_signals());

        let config = config::load_from_paths_with_provider(&config_paths_with_formats(), &mut signal_handler)
            .await
            .map_err(handle_config_errors)
            .unwrap();
        let log_schema = config.global.log_schema.clone();
        config::init_log_schema(|| {
            Ok(log_schema)
        }, true);

        let diff = config::ConfigDiff::initial(&config);
        let pieces = topology::build_or_log_errors(&config, &diff, HashMap::new())
            .await
            .ok_or(exitcode::CONFIG)
            .unwrap();
        let result = topology::start_validate(config, diff, pieces).await;
        let (mut topology, graceful_crash) = result.ok_or(exitcode::CONFIG).unwrap();

        // run
        let mut graceful_crash = UnboundedReceiverStream::new(graceful_crash);
        let mut sources_finished = topology.sources_finished();

        let signal = loop {
            tokio::select! {
                    Some(signal) = signal_rx.recv() => {
                        match signal {
                            SignalTo::ReloadFromConfigBuilder(builder) => {
                                match builder.build().map_err(handle_config_errors) {
                                    Ok(mut new_config) => {
                                        new_config.health_checks.set_require_healthy(true);
                                        match topology.reload_config_and_respawn(new_config).await {
                                            Ok(true) => {
                                                info!("Vertex reloaded");
                                            },
                                            Ok(false) => {
                                                info!("Vertex reload failed");
                                            },
                                            // Trigger graceful shutdown for what remains of the topology
                                            Err(()) => {
                                                break SignalTo::Shutdown;
                                            }
                                        }

                                        sources_finished = topology.sources_finished();
                                    },

                                    Err(_) => {
                                        warn!("Vertex config reload failed");
                                    }
                                }
                            }

                            SignalTo::ReloadFromDisk => {
                                // Reload paths
                                let config_paths = config_paths_with_formats();
                                let new_config = config::load_from_paths_with_provider(&config_paths, &mut signal_handler).await
                                    .map_err(handle_config_errors)
                                    .ok();

                                if let Some(mut new_config) = new_config {
                                    new_config.health_checks.set_require_healthy(true);
                                    match topology.reload_config_and_respawn(new_config).await {
                                        Ok(true) => {
                                            info!("Reload config successes");
                                        },
                                        Ok(false) => {
                                            warn!("Reload config failed");
                                        },
                                        Err(()) => {
                                            break SignalTo::Shutdown;
                                        }
                                    }

                                    sources_finished = topology.sources_finished();
                                } else {
                                    warn!("Reload config failed");
                                }
                            }

                            _ => break signal,
                        }
                    },

                    // Trigger graceful shutdown if a component crashed, or all sources have ended
                    _ = graceful_crash.next() => break SignalTo::Shutdown,
                    _ = &mut sources_finished => break SignalTo::Shutdown,
                    else => unreachable!("Signal streams never end"),
                }
        };

        match signal {
            SignalTo::Shutdown => {
                info!("Shutdown signal received");

                tokio::select! {
                        // graceful shutdown finished
                        _ = topology.stop() => (),
                        _ = signal_rx.recv() => {
                            // it is highly unlikely that this event will exit from topology

                            // Dropping the shutdown future will immediately shut the server down
                        }
                    }
            }

            SignalTo::Quit => {
                info!("Quit signal received");

                // It is highly unlikely that this event will exit from topology
                drop(topology);
            }

            _ => unreachable!(),
        }
    });

    rt.shutdown_timeout(Duration::from_secs(5))
}

pub fn handle_config_errors(errors: Vec<String>) -> exitcode::ExitCode {
    for err in errors {
        error!(
            message = "configuration error",
            ?err
        );
    }

    exitcode::CONFIG
}

// TODO: implement it
fn config_paths_with_formats() -> Vec<config::ConfigPath> {
    vec![ConfigPath::File("dev.yml".into(), FormatHint::from(config::Format::YAML))]
}