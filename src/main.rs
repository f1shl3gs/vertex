extern crate vertex;

// mimalloc do not help for reducing memory usage,
//
// use mimalloc::MiMalloc;
//
// #[global_allocator]
// static GLOBAL: MiMalloc = MiMalloc;

use tokio::runtime;
use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use tokio::time::Duration;

#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_scope;
extern crate slog_term;

use clap::{AppSettings, Clap};

use slog::Drain;
use vertex::{
    signal::{self, SignalTo},
    config::{self, ConfigPath, FormatHint},
    topology,
};
use std::collections::HashMap;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;


async fn handle(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let guard = pprof::ProfilerGuard::new(100).unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

    match guard.report().build() {
        Ok(report) => {
            let file = std::fs::File::create("flamegraph.svg").unwrap();
            report.flamegraph(file).unwrap();
        }

        Err(_) => {}
    }

    Ok(Response::new(Body::from(vec![])))
}

fn setup_logger() -> slog::Logger {
    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let drain = slog_term::FullFormat::new(plain).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    slog::Logger::root(drain, o!())
}

#[derive(Clap, Debug)]
#[clap(version = "0.1.0")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(short, long, default_value = "/etc/vertex/vertex.conf")]
    pub config: String,

    // todo sub commands
}

fn main() {
    let opts: Opts = Opts::parse();

    let rt = runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    rt.spawn(async {
        let addr = "127.0.0.1:3080".parse().expect("listen addr");

        let make_service = make_service_fn(|_conn| async {
            Ok::<_, Infallible>(service_fn(handle))
        });

        let svr = Server::bind(&addr)
            .serve(make_service);

        if let Err(e) = svr.await {
            eprintln!("server error: {}", e)
        }
    });

    let logger = setup_logger();
    // Make sure to save the guard, see documentation for more information
    let _guard = slog_scope::set_global_logger(logger);
    slog_scope::scope(&slog_scope::logger().new(o!()), || {
        info!("start vertex"; "config" => opts.config);

        rt.block_on(async move {
            let (mut signal_handler, mut signal_rx) = signal::SignalHandler::new();
            signal_handler.forever(signal::os_signals());

            let config = config::load_from_paths_with_provider(&config_paths_with_formats(), &mut signal_handler)
                .await
                .map_err(handle_config_errors)
                .unwrap();

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

        info!("Trying to shutdown Tokio runtime");
        rt.shutdown_timeout(Duration::from_secs(5))
    });
}

pub fn handle_config_errors(errors: Vec<String>) -> exitcode::ExitCode {
    for err in errors {
        error!("configuration error"; "err" => err);
    }

    exitcode::CONFIG
}

// TODO: implement it
fn config_paths_with_formats() -> Vec<config::ConfigPath> {
    vec![ConfigPath::File("dev.yml".into(), FormatHint::from(config::Format::YAML))]
}