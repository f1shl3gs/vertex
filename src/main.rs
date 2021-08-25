extern crate vertex;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use tokio::runtime;
use num_cpus;
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
use std::path::PathBuf;
use vertex::{
    signal,
    config,
    topology,
};
use std::collections::HashMap;
use tokio_stream::wrappers::UnboundedReceiverStream;
use vertex::signal::SignalTo;
use vertex::config::{ConfigPath, FormatHint};
use tokio_stream::StreamExt;


async fn handle(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    info!("handle request"; "log-key" => true);
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
    println!("{:?}", opts);

    let workers = num_cpus::get();

    let rt = runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .thread_name("vertex-worker")
        .thread_stack_size(4 * 1024 * 1023)
        .enable_io()
        .build()
        .unwrap();

    let logger = setup_logger();
    // Make sure to save the guard, see documentation for more information
    let _guard = slog_scope::set_global_logger(logger);
    slog_scope::scope(&slog_scope::logger().new(o!()), || {
        info!("start vertex"; "workers" => workers, "config" => opts.config);

        rt.block_on(async move{
            let (mut signal_handler, signal_rx) = signal::SignalHandler::new();
            signal_handler.forever(signal::os_signals());

            let config = config::load_from_paths_with_provider(&config_paths_with_formats(), &mut signal_handler)
                .await
                .map_err(handle_config_errors)?;

            let diff = config::ConfigDiff::initial(&config);
            let pieces = topology::build_or_log_errors(&config, &diff, HashMap::new()).await.ok_or(exitcode::CONFIG)?;
            let result = topology::start_validate(config, diff, pieces).await;
            let (mut topology, graceful_crash) = result.ok_or(exitcode::CONFIG)?;

            // run
            let mut graceful_crash = UnboundedReceiverStream::new(graceful_crash);
            let mut sources_finished = topology.sources_finished();

            let signal = loop {
                tokio::select! {
                    Some(signal) = signal_rx.recv() => {

                    },
                    // Trigger graceful shutdown if a component crashed, or all sources have ended
                    _ = graceful_crash.next() => break SignalTo::Shutdown,
                    _ = &mut sources_finished => break SignalTo::Shutdown,
                    else => unreachable!("Signal streams never end"),
                }
            };

            match signal {
                SignalTo::Shutdown => {
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
                    // It is highly unlikely that this event will exit from topology
                    drop(topology);
                }

                _ => unreachable!(),
            }

            Ok(())
        });

        /*
        rt.block_on(async {
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
        */

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