use std::collections::HashMap;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

use argh::FromArgs;
use configurable::component::{
    ExtensionDescription, ProviderDescription, SinkDescription, SourceDescription,
    TransformDescription,
};
use exitcode::ExitCode;
use framework::{config, signal, topology, SignalTo};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use vertex::built_info::{GIT_HASH, PKG_VERSION, RUSTC_VERSION, TARGET};
#[cfg(feature = "extensions-healthcheck")]
use vertex::extensions::healthcheck;
#[cfg(feature = "extensions-heartbeat")]
use vertex::extensions::heartbeat;

use crate::{top, validate};

fn default_worker_threads() -> usize {
    std::thread::available_parallelism()
        .expect("get available working threads")
        .get()
}

#[derive(FromArgs)]
#[argh(description = "Vertex is an all-in-one collector for metrics, logs and traces")]
pub struct RootCommand {
    #[argh(switch, short = 'v', description = "show version")]
    pub version: bool,

    #[argh(
        option,
        short = 'l',
        default = "\"info\".to_string()",
        description = "log level"
    )]
    pub log_level: String,

    #[argh(
        option,
        short = 'c',
        long = "config",
        description = "read configuration from one or more files, wildcard paths are supported"
    )]
    pub configs: Vec<PathBuf>,

    #[cfg(all(unix, not(target_os = "macos")))]
    #[argh(
        switch,
        short = 'w',
        long = "watch",
        description = "watch config files and reload when it changed"
    )]
    pub watch: bool,

    #[argh(
        option,
        short = 't',
        default = "default_worker_threads()",
        description = "specify how many threads the Tokio runtime will use"
    )]
    pub threads: usize,

    #[argh(
        option,
        default = "20",
        description = "specify keepalive of blocking threads, in seconds"
    )]
    pub blocking_thread_keepalive: u64,

    #[argh(
        option,
        default = "512",
        description = "specifies the limit for additional blocking threads spawned by the Runtime"
    )]
    pub max_blocking_threads: usize,

    #[argh(subcommand)]
    pub sub_commands: Option<SubCommands>,
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

        let levels =
            std::env::var("VERTEX_LOG").unwrap_or_else(|_| match self.log_level.as_str() {
                "off" => "off".to_owned(),
                #[cfg(feature = "tokio-console")]
                level => [
                    format!("vertex={}", level),
                    format!("framework={}", level),
                    format!("tail={}", level),
                    format!("codec={}", level),
                    format!("tail={}", level),
                    "tower_limit=trace".to_owned(),
                    "runtime=trace".to_owned(),
                    "tokio=trace".to_owned(),
                    format!("rskafka={}", level),
                    format!("buffers={}", level),
                ]
                .join(","),
                #[cfg(not(feature = "tokio-console"))]
                level => [
                    format!("vertex={}", level),
                    format!("framework={}", level),
                    format!("tail={}", level),
                    format!("codec={}", level),
                    format!("file_source={}", level),
                    "tower_limit=trace".to_owned(),
                    format!("rskafka={}", level),
                    format!("buffers={}", level),
                ]
                .join(","),
            });

        #[cfg(unix)]
        let color = std::io::stdout().is_terminal();
        #[cfg(not(unix))]
        let color = false;
        framework::trace::init(color, false, &levels, 10);

        // Note: `block_on` will spawn another worker thread too. so actual running
        // threads is always >= threads + 1.
        runtime.block_on(async move {
            let mut config_paths = config::process_paths(&config_paths).ok_or(exitcode::CONFIG)?;

            info!(
                message = "Start vertex",
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
                config::watcher::spawn_thread(config_paths.iter().map(Into::into), None)
                    .map_err(|err| {
                        error!(
                        message = "Unable to start config watcher",
                        ?err
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
                                    new_config.healthcheck.set_require_healthy(true);
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

            #[cfg(feature = "extensions-healthcheck")]
            healthcheck::set_readiness(false);

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

            Ok::<(), ExitCode>(())
        })?;

        runtime.shutdown_timeout(Duration::from_secs(5));

        Ok(())
    }
}

pub fn handle_config_errors(errors: Vec<String>) -> ExitCode {
    for err in errors {
        error!(message = "configuration error", ?err);
    }

    exitcode::CONFIG
}

#[derive(Debug, FromArgs)]
#[argh(subcommand)]
pub enum SubCommands {
    Sources(Sources),
    Transforms(Transforms),
    Sinks(Sinks),
    Extensions(Extensions),
    Providers(Providers),
    Validate(validate::Validate),
    Top(top::Top),
}

impl SubCommands {
    pub fn run(&self) -> Result<(), ExitCode> {
        match self {
            SubCommands::Sources(sources) => sources.run(),
            SubCommands::Transforms(transforms) => transforms.run(),
            SubCommands::Sinks(sinks) => sinks.run(),
            SubCommands::Extensions(extensions) => extensions.run(),
            SubCommands::Providers(providers) => providers.run(),
            SubCommands::Validate(validate) => match validate.run() {
                exitcode::OK => Ok(()),
                other => Err(other),
            },
            SubCommands::Top(top) => top.run(),
        }
    }
}

macro_rules! impl_list_and_example {
    ($typ:ident, $desc:ident) => {
        impl $typ {
            #![allow(clippy::print_stdout)]
            pub fn run(&self) -> Result<(), ExitCode> {
                match &self.name {
                    Some(name) => match $desc::example(&name) {
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
                        for item in $desc::types() {
                            println!("{}", item);
                        }

                        Ok(())
                    }
                }
            }
        }
    };
}

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "sources", description = "List all sources")]
pub struct Sources {
    #[argh(positional, description = "source name")]
    name: Option<String>,
}

impl_list_and_example!(Sources, SourceDescription);

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "transforms", description = "List transforms")]
pub struct Transforms {
    #[argh(positional, description = "transform name")]
    name: Option<String>,
}

impl_list_and_example!(Transforms, TransformDescription);

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "sinks", description = "List sinks")]
pub struct Sinks {
    #[argh(positional, description = "sink name")]
    name: Option<String>,
}

impl_list_and_example!(Sinks, SinkDescription);

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "extensions", description = "List extensions")]
pub struct Extensions {
    #[argh(positional, description = "extension name")]
    name: Option<String>,
}

impl_list_and_example!(Extensions, ExtensionDescription);

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "providers", description = "List providers")]
pub struct Providers {
    #[argh(positional, description = "provider name")]
    name: Option<String>,
}

impl_list_and_example!(Providers, ProviderDescription);
