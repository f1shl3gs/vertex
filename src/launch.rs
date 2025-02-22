use std::collections::HashMap;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

use argh::FromArgs;
use exitcode::ExitCode;
use framework::{SignalTo, config, signal, topology};
use futures::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{error, info, warn};
use vertex::built_info::{GIT_HASH, PKG_VERSION, RUSTC_VERSION, TARGET};
#[cfg(feature = "extensions-healthcheck")]
use vertex::extensions::healthcheck;
#[cfg(feature = "extensions-heartbeat")]
use vertex::extensions::heartbeat;
#[cfg(feature = "extensions-zpages")]
use vertex::extensions::zpages;

use crate::{top, validate, vtl};

fn default_worker_threads() -> usize {
    match std::env::var("VERTEX_WORKER_THREADS") {
        Ok(value) => value
            .parse::<usize>()
            .expect("invalid env value for VERTEX_WORKER_THREADS"),
        Err(_) => {
            // not found
            std::thread::available_parallelism()
                .expect("get available working threads")
                .get()
        }
    }
}

fn default_max_blocking_threads() -> usize {
    match std::env::var("VERTEX_MAX_BLOCKING_THREADS") {
        Ok(value) => value
            .parse::<usize>()
            .expect("invalid env value for VERTEX_MAX_BLOCKING_THREADS"),
        Err(_) => {
            // not found, use default value
            256usize
        }
    }
}

#[derive(FromArgs)]
#[argh(
    description = "Vertex is an all-in-one collector for metrics, logs and traces",
    help_triggers("-h", "--help")
)]
pub struct RootCommand {
    #[argh(switch, short = 'v', description = "show version")]
    version: bool,

    #[argh(
        option,
        short = 'l',
        default = "\"info\".to_string()",
        description = "log level"
    )]
    log_level: String,

    #[argh(
        option,
        short = 'c',
        long = "config",
        description = "read configuration from one or more files, wildcard paths are supported"
    )]
    configs: Vec<PathBuf>,

    #[cfg(all(unix, not(target_os = "macos")))]
    #[argh(
        switch,
        short = 'w',
        long = "watch",
        description = "watch config files and reload when it changed"
    )]
    watch: bool,

    #[argh(
        option,
        short = 't',
        default = "default_worker_threads()",
        description = "specify how many threads the Tokio runtime will use"
    )]
    threads: usize,

    #[argh(
        option,
        default = "20",
        description = "specify keepalive of blocking threads, in seconds"
    )]
    blocking_thread_keepalive: u64,

    #[argh(
        option,
        default = "default_max_blocking_threads()",
        description = "specifies the limit for additional blocking threads spawned by the Runtime"
    )]
    max_blocking_threads: usize,

    #[argh(subcommand)]
    sub_commands: Option<SubCommands>,
}

impl RootCommand {
    #![allow(clippy::print_stdout)]
    fn show_version(&self) {
        println!("Vertex {} -- {}", PKG_VERSION, GIT_HASH);
        println!("Target {}", TARGET);
        println!("rustc  {}", RUSTC_VERSION);
    }

    fn config_paths_with_formats(&self) -> Vec<config::ConfigPath> {
        config::merge_path_lists(vec![(&self.configs, None)])
            .map(|(path, hint)| config::ConfigPath::File(path, hint))
            .collect::<Vec<_>>()
    }

    pub fn run(&self) -> Result<(), ExitCode> {
        if self.version {
            self.show_version();
            return Ok(());
        }

        if let Some(sub_command) = &self.sub_commands {
            sub_command.run()?;
            return Ok(());
        }

        let config_paths = self.config_paths_with_formats();
        #[cfg(all(unix, not(target_os = "macos")))]
        let watch_config = self.watch;

        // set workers, so other component can read this
        framework::set_workers(self.threads);

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .thread_name("vertex-worker")
            .worker_threads(self.threads)
            .max_blocking_threads(self.max_blocking_threads)
            // default interval of sources is 15s, 30s should reduce some overhead for
            // create/destroy threads.
            .thread_keep_alive(Duration::from_secs(self.blocking_thread_keepalive))
            .enable_io()
            .enable_time()
            .build()
            .unwrap();

        let log_level = std::env::var("VERTEX_LOG").unwrap_or(self.log_level.clone());
        let color = std::io::stdout().is_terminal();
        framework::trace::init(color, false, &log_level, 10);

        // Note: `block_on` will spawn another worker thread too. so actual running
        // threads is always >= threads + 1.
        runtime.block_on(async move {
            let mut config_paths = config::process_paths(&config_paths).ok_or(exitcode::CONFIG)?;

            let allocator = if cfg!(feature = "jemalloc") {
                "jemalloc"
            } else if cfg!(feature = "snmalloc") {
                "snmalloc"
            } else if cfg!(feature = "mimalloc") {
                "mimalloc"
            } else if cfg!(feature = "scudo") {
                "scudo"
            } else if cfg!(feature = "tracked_alloc") {
                "tracked_alloc"
            } else {
                "system"
            };

            info!(
                message = "Start vertex",
                allocator,
                threads = self.threads,
                max_blocking_threads = self.max_blocking_threads,
                configs = ?config_paths
            );

            let (mut signal_handler, mut signal_rx) = signal::SignalHandler::new();
            signal_handler.forever(signal::os_signals());

            let config = config::load_from_paths_with_provider(&config_paths, &mut signal_handler)
                .await
                .map_err(handle_config_errors)?;

            #[cfg(all(unix, not(target_os = "macos")))]
            if watch_config {
                // Start listening for config changes immediately.
                config::watcher::watch_configs(config_paths.iter().map(Into::into))
                    .map_err(|err| {
                        error!(
                            message = "Unable to start config watcher",
                            %err
                        );

                        exitcode::CONFIG
                    })?;
            }

            // TODO: how to set this when reload
            let schema = config.global.log_schema.clone();
            log_schema::init_log_schema(|| Ok(schema), true)
                .expect("init log schema success");

            let diff = config::ConfigDiff::initial(&config);
            let pieces = topology::build_or_log_errors(&config, &diff, HashMap::new())
                .await
                .ok_or(exitcode::CONFIG)?;
            let result = topology::start_validate(config, diff, pieces).await;

            #[cfg(feature = "extensions-healthcheck")]
            healthcheck::set_readiness(true);

            let (mut topology, graceful_crash) = result.ok_or(exitcode::CONFIG).unwrap();

            #[cfg(feature = "extensions-heartbeat")]
            heartbeat::report_config(topology.config());
            #[cfg(feature = "extensions-zpages")]
            zpages::update_config(topology.config());

            // run
            let mut graceful_crash = UnboundedReceiverStream::new(graceful_crash);
            let mut sources_finished = topology.sources_finished();

            // Any internal_logs source will have grabbed a copy of the early buffer by this
            // point and set up a subscriber
            framework::trace::stop_buffering();

            let signal = loop {
                tokio::select! {
                    Some(signal) = signal_rx.recv() => {
                        match signal {
                            SignalTo::ReloadFromConfigBuilder(builder) => {
                                match builder.build().map_err(handle_config_errors) {
                                    Ok(mut new_config) => {
                                        new_config.healthcheck.set_require_healthy(true);
                                        match topology.reload_config_and_respawn(new_config).await {
                                            Ok(true) => {
                                                #[cfg(feature = "extensions-heartbeat")]
                                                heartbeat::report_config(topology.config());
                                                #[cfg(feature = "extensions-zpages")]
                                                zpages::update_config(topology.config());

                                                info!(message = "Vertex reloaded");
                                            },
                                            Ok(false) => {
                                                info!(message = "Vertex reload failed");
                                            },
                                            // Trigger graceful shutdown for what remains of the topology
                                            Err(()) => {
                                                break SignalTo::Shutdown;
                                            }
                                        }

                                        sources_finished = topology.sources_finished();
                                    },

                                    Err(_) => {
                                        warn!(message = "Vertex config reload failed");
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
                                    new_config.healthcheck.set_require_healthy(true);
                                    match topology.reload_config_and_respawn(new_config).await {
                                        Ok(true) => {
                                            #[cfg(feature = "extensions-heartbeat")]
                                            heartbeat::report_config(topology.config());
                                            #[cfg(feature = "extensions-zpages")]
                                            zpages::update_config(topology.config());

                                            info!(message = "Reload config successes");
                                        },
                                        Ok(false) => {
                                            warn!(message = "Reload config failed");
                                        },
                                        Err(()) => {
                                            break SignalTo::Shutdown;
                                        }
                                    }

                                    sources_finished = topology.sources_finished();
                                } else {
                                    warn!(message = "Reload config failed");
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

            #[cfg(feature = "extensions-healthcheck")]
            healthcheck::set_readiness(false);

            match signal {
                SignalTo::Shutdown => {
                    info!(message = "Shutdown signal received");

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
                    info!(message = "Quit signal received");

                    // It is highly unlikely that this event will exit from topology
                    drop(topology);
                }

                _ => unreachable!(),
            }

            Ok::<(), ExitCode>(())
        })?;

        runtime.shutdown_timeout(Duration::from_secs(5));

        Ok(())
    }
}

pub fn handle_config_errors(errors: Vec<String>) -> ExitCode {
    for err in errors {
        error!(message = "configuration error", %err);
    }

    exitcode::CONFIG
}

#[derive(Debug, FromArgs)]
#[argh(
    subcommand,
    name = "sources",
    description = "List sources",
    help_triggers("-h", "--help")
)]
struct Sources {
    #[argh(positional, description = "source name")]
    name: Option<String>,
}

#[derive(Debug, FromArgs)]
#[argh(
    subcommand,
    name = "transforms",
    description = "List transforms",
    help_triggers("-h", "--help")
)]
struct Transforms {
    #[argh(positional, description = "transform name")]
    name: Option<String>,
}

#[derive(Debug, FromArgs)]
#[argh(
    subcommand,
    name = "sinks",
    description = "List sinks",
    help_triggers("-h", "--help")
)]
struct Sinks {
    #[argh(positional, description = "sink name")]
    name: Option<String>,
}

#[derive(Debug, FromArgs)]
#[argh(
    subcommand,
    name = "extensions",
    description = "List extensions",
    help_triggers("-h", "--help")
)]
struct Extensions {
    #[argh(positional, description = "extension name")]
    name: Option<String>,
}

#[derive(Debug, FromArgs)]
#[argh(
    subcommand,
    name = "providers",
    description = "List providers",
    help_triggers("-h", "--help")
)]
struct Providers {
    #[argh(positional, description = "provider name")]
    name: Option<String>,
}

macro_rules! list_or_example {
    ($name:expr, $desc:ident) => {
        match $name {
            Some(name) => match configurable::component::$desc::example(&name) {
                Ok(example) => {
                    println!("{}", example.trim());
                    Ok(())
                }
                Err(err) => {
                    println!("Generate example failed: {}", err);
                    Err(exitcode::USAGE)
                }
            },

            _ => {
                for item in configurable::component::$desc::types() {
                    println!("{}", item);
                }

                Ok(())
            }
        }
    };
}

#[derive(Debug, FromArgs)]
#[argh(subcommand)]
enum SubCommands {
    Sources(Sources),
    Transforms(Transforms),
    Sinks(Sinks),
    Extensions(Extensions),
    Providers(Providers),
    Validate(validate::Validate),
    Top(top::Top),
    Vtl(vtl::Vtl),
}

impl SubCommands {
    fn run(&self) -> Result<(), ExitCode> {
        match self {
            SubCommands::Sources(sources) => list_or_example!(&sources.name, SourceDescription),
            SubCommands::Transforms(transforms) => {
                list_or_example!(&transforms.name, TransformDescription)
            }
            SubCommands::Sinks(sinks) => list_or_example!(&sinks.name, SinkDescription),
            SubCommands::Extensions(extensions) => {
                list_or_example!(&extensions.name, ExtensionDescription)
            }
            SubCommands::Providers(providers) => {
                list_or_example!(&providers.name, ProviderDescription)
            }
            SubCommands::Validate(validate) => match validate.run() {
                exitcode::OK => Ok(()),
                other => Err(other),
            },
            SubCommands::Top(top) => top.run(),
            SubCommands::Vtl(vtl) => vtl.run(),
        }
    }
}
