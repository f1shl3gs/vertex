mod commands;

extern crate vertex;

#[cfg(feature = "allocator-mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(any(feature = "allocator-jemalloc", feature = "extensions-jemalloc"))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

extern crate chrono;
extern crate chrono_tz;

use crate::commands::RootCommand;
use std::collections::HashMap;
use tokio::time::Duration;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use vertex::{
    config,
    signal::{self, SignalTo},
    topology,
};

fn main() {
    let opts: RootCommand = argh::from_env();

    if opts.version {
        opts.show_version();
        return;
    }

    if let Some(commands) = opts.sub_commands {
        commands.run();
        return;
    }

    let mut config_paths = opts.config_paths_with_formats();
    let threads = opts.threads.unwrap_or_else(num_cpus::get);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(threads)
        .thread_name("vertex-worker")
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    let levels = std::env::var("VERTEX_LOG").unwrap_or_else(|_| match opts.log_level.as_str() {
        "off" => "off".to_owned(),
        #[cfg(feature = "tokio-console")]
        level => [
            format!("vertex={}", level),
            format!("codec={}", level),
            format!("tail={}", level),
            "tower_limit=trace".to_owned(),
            "runtime=trace".to_owned(),
            "tokio=trace".to_owned(),
            format!("rdkafka={}", level),
            format!("buffers={}", level),
        ]
        .join(","),
        #[cfg(not(feature = "tokio-console"))]
        level => [
            format!("vertex={}", level),
            format!("codec={}", level),
            format!("vrl={}", level),
            format!("file_source={}", level),
            "tower_limit=trace".to_owned(),
            format!("rdkafka={}", level),
            format!("buffers={}", level),
        ]
        .join(","),
    });

    runtime.block_on(async move {
        vertex::trace::init(true, false, &levels);

        info!(
            message = "start vertex",
            threads = threads,
            configs = ?opts.configs
        );

        openssl_probe::init_ssl_cert_env_vars();

        let (mut signal_handler, mut signal_rx) = signal::SignalHandler::new();
        signal_handler.forever(signal::os_signals());

        let config = config::load_from_paths_with_provider(&config_paths, &mut signal_handler)
            .await
            .map_err(handle_config_errors)
            .unwrap();

        // TODO: how to set this when reload
        let schema = config.global.log_schema.clone();
        log_schema::init_log_schema(|| Ok(schema), true)
            .expect("init log schema success");

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

        // Any internal_logs source will have grabbed a copy of the early buffer by this
        // point and set up a subscriber
        vertex::trace::stop_buffering();

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
                                config_paths = config::process_paths(&config_paths).unwrap_or(config_paths);
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

    runtime.shutdown_timeout(Duration::from_secs(5))
}

pub fn handle_config_errors(errors: Vec<String>) -> exitcode::ExitCode {
    for err in errors {
        error!(message = "configuration error", ?err);
    }

    exitcode::CONFIG
}
