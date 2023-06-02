use std::collections::HashMap;
use std::fmt;
use std::fs::remove_dir_all;
use std::io::IsTerminal;
use std::path::PathBuf;

use argh::FromArgs;
use exitcode::ExitCode;
use framework::config;
use framework::config::{load_builder_from_paths, Config, ConfigDiff, ConfigPath};
use framework::topology::{build_pieces, take_healthchecks, Pieces};
use tracing::error;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(FromArgs, PartialEq, Debug)]
#[argh(
    subcommand,
    name = "validate",
    description = "Validate target configs, then exit"
)]
pub struct Validate {
    #[argh(
        switch,
        short = 'd',
        description = "fail validation on warnings that are probably a mistake in the configuration or are recommended to be fixed"
    )]
    deny_warnings: bool,

    #[argh(
        switch,
        description = "disable environment checks. that includes component checks and health checks"
    )]
    no_environment: bool,

    #[argh(
        option,
        short = 'c',
        description = "read configuration from files in one or more directories"
    )]
    configs: Vec<PathBuf>,
}

impl Validate {
    pub fn run(&self) -> ExitCode {
        #[cfg(unix)]
        let color = std::io::stdout().is_terminal();
        #[cfg(not(unix))]
        let color = false;

        let mut fmt = Formatter::new(color);
        let mut validated = true;
        let mut config = match self.validate_config(&mut fmt) {
            Some(config) => config,
            None => return exitcode::CONFIG,
        };

        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .enable_time()
            .build()
        {
            Ok(rt) => rt,
            Err(_) => return exitcode::CANTCREAT,
        };

        rt.block_on(async move {
            if !self.no_environment {
                if let Some(tmp_dir) = create_tmp_directory(&mut config, &mut fmt) {
                    validated &= self.validate_environment(&config, &mut fmt).await;
                    remove_tmp_directory(tmp_dir);
                } else {
                    validated = false;
                }
            }

            if validated {
                fmt.validated();
                exitcode::OK
            } else {
                exitcode::CONFIG
            }
        })
    }

    fn validate_config(&self, fmt: &mut Formatter) -> Option<Config> {
        let paths = config::merge_path_lists(vec![(&self.configs, None)])
            .map(|(path, hint)| config::ConfigPath::File(path, hint))
            .collect::<Vec<_>>();

        let paths = if let Some(paths) = config::process_paths(&paths) {
            paths
        } else {
            fmt.error("No config file paths");
            return None;
        };

        // Load
        let paths_list: Vec<_> = paths.iter().map(<&PathBuf>::from).collect();

        let mut report_error = |errs| {
            fmt.title(format!("Failed to load {:?}", &paths_list));
            fmt.sub_error(errs)
        };

        init_log_schema(&paths, true)
            .map_err(&mut report_error)
            .ok()?;
        let (builder, load_warnings) = config::load_builder_from_paths(&paths)
            .map_err(&mut report_error)
            .ok()?;

        // Build
        let (config, build_warnings) = builder
            .build_with_warnings()
            .map_err(&mut report_error)
            .ok()?;

        // Warnings
        let warnings = load_warnings
            .into_iter()
            .chain(build_warnings)
            .collect::<Vec<_>>();
        if !warnings.is_empty() {
            if self.deny_warnings {
                report_error(warnings);
                return None;
            }

            fmt.title(format!("Loaded with warnings {:?}", &paths_list));
            fmt.sub_warning(warnings);
        } else {
            fmt.success(format!("Loaded {:?}", &paths_list));
        }

        Some(config)
    }

    async fn validate_environment(&self, config: &Config, fmt: &mut Formatter) -> bool {
        let diff = config::ConfigDiff::initial(config);

        let mut pieces = if let Some(pieces) = validate_components(config, &diff, fmt).await {
            pieces
        } else {
            return false;
        };

        self.validate_healthchecks(config, &diff, &mut pieces, fmt)
            .await
    }

    async fn validate_healthchecks(
        &self,
        config: &Config,
        diff: &ConfigDiff,
        pieces: &mut Pieces,
        fmt: &mut Formatter,
    ) -> bool {
        if !config.healthchecks.enabled {
            fmt.warning("Health checks are disabled");
            return !self.deny_warnings;
        }

        let healthchecks = take_healthchecks(diff, pieces);
        // We are running health checks in serial so it's easier for the users
        // to parse which errors/warnings/etc. belong to which healthcheck.
        let mut validated = true;
        for (id, healthcheck) in healthchecks {
            let mut failed = |err| {
                validated = false;
                fmt.error(err);
            };

            match tokio::spawn(healthcheck).await {
                Ok(Ok(_)) => {
                    if config
                        .sinks
                        .get(&id)
                        .expect("Sink not present")
                        .health_check()
                    {
                        fmt.success(format!("Health check \"{}\"", id));
                    } else {
                        fmt.warning(format!("Health check disabled for \"{}\"", id));
                        validated &= !self.deny_warnings;
                    }
                }

                Ok(Err(())) => failed(format!("Health check for \"{}\" failed", id)),

                Err(err) if err.is_cancelled() => {
                    failed(format!("Health check for \"{}\" was cancelled", id))
                }

                Err(_) => failed(format!("Health check for \"{}\" panicked", id)),
            }
        }

        validated
    }
}

/// Loads Log Schema from configurations and sets global schema.
/// Once this is done, configurations can be correctly loaded using
/// configured log schema defaults. If deny is set, will panic if
/// schema has already been set.
fn init_log_schema(paths: &[ConfigPath], deny_if_set: bool) -> Result<(), Vec<String>> {
    log_schema::init_log_schema(
        || {
            let (builder, _) = load_builder_from_paths(paths)?;
            Ok(builder.global.log_schema)
        },
        deny_if_set,
    )
}

async fn validate_components(
    config: &Config,
    diff: &ConfigDiff,
    fmt: &mut Formatter,
) -> Option<Pieces> {
    match build_pieces(config, diff, HashMap::new()).await {
        Ok(pieces) => {
            fmt.success("Component configuration");
            Some(pieces)
        }

        Err(errs) => {
            fmt.title("Component errors");
            fmt.sub_error(errs);
            None
        }
    }
}

const TEMPORARY_DIRECTORY: &str = "validate_tmp";

/// For data directory that we write to:
/// 1. Create a tmp directory in it.
/// 2. Change config to point to that tmp directory
fn create_tmp_directory(config: &mut Config, fmt: &mut Formatter) -> Option<PathBuf> {
    match config.global.make_subdir(TEMPORARY_DIRECTORY) {
        Ok(path) => {
            config.global.data_dir = Some(path.clone());
            Some(path)
        }
        Err(err) => {
            fmt.error(format!("{:?}", err));
            None
        }
    }
}

fn remove_tmp_directory(path: PathBuf) {
    if let Err(err) = remove_dir_all(&path) {
        error!(
            message = "Failed to remove temporary directory",
            path = ?path,
            %err
        );
    }
}

struct Formatter {
    /// Width of largest printed line
    max_line_width: usize,
    /// Can empty line be printed
    print_space: bool,
    color: bool,
    // Intros
    error_intro: &'static str,
    warning_intro: &'static str,
    success_intro: &'static str,
}

impl Formatter {
    fn new(color: bool) -> Self {
        Self {
            max_line_width: 0,
            print_space: false,
            error_intro: if color {
                // red
                "\x1b[31mx\x1b[0m"
            } else {
                "x"
            },
            warning_intro: if color {
                // yellow
                "\x1b[33m~\x1b[0m"
            } else {
                "~"
            },
            success_intro: if color {
                // green
                "\x1b[32m√\x1b[0m"
            } else {
                "√"
            },
            color,
        }
    }

    /// Final confirmation that validation process was successful.
    #[allow(clippy::print_stdout)]
    fn validated(&self) {
        println!("{:-^width$}", "", width = self.max_line_width);

        if self.color {
            // Coloring needs to be used directly so that print
            // infrastructure correctly determines length of the
            // "Validated". Otherwise, ansi escape coloring is
            // calculated into the length.
            println!(
                "{:>width$}",
                "\x1b[32mValidated\x1b[0m", // green
                width = self.max_line_width
            );
        } else {
            println!("{:>width$}", "Validated", width = self.max_line_width)
        }
    }

    /// Standalone line
    fn success(&mut self, msg: impl AsRef<str>) {
        self.print(format!("{} {}\n", self.success_intro, msg.as_ref()))
    }

    /// Standalone line
    fn warning(&mut self, warning: impl AsRef<str>) {
        self.print(format!("{} {}\n", self.warning_intro, warning.as_ref()))
    }

    /// Standalone line
    fn error(&mut self, error: impl AsRef<str>) {
        self.print(format!("{} {}\n", self.error_intro, error.as_ref()))
    }

    /// Marks sub
    fn title(&mut self, title: impl AsRef<str>) {
        self.space();
        self.print(format!(
            "{}\n{:-<width$}\n",
            title.as_ref(),
            "",
            width = title.as_ref().len()
        ))
    }

    /// A list of warnings that go with a title.
    fn sub_warning<I: IntoIterator>(&mut self, warnings: I)
    where
        I::Item: fmt::Display,
    {
        self.sub(self.warning_intro, warnings)
    }

    /// A list of errors that go with a title.
    fn sub_error<I: IntoIterator>(&mut self, errors: I)
    where
        I::Item: fmt::Display,
    {
        self.sub(self.error_intro, errors)
    }

    fn sub<I: IntoIterator>(&mut self, intro: impl AsRef<str>, msgs: I)
    where
        I::Item: fmt::Display,
    {
        for msg in msgs {
            self.print(format!("{} {}\n", intro.as_ref(), msg));
        }
        self.space();
    }

    /// Prints empty space if necessary.
    fn space(&mut self) {
        if self.print_space {
            self.print_space = false;
            #[allow(clippy::print_stdout)]
            {
                println!();
            }
        }
    }

    fn print(&mut self, print: impl AsRef<str>) {
        let width = print
            .as_ref()
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        self.max_line_width = width.max(self.max_line_width);
        self.print_space = true;
        #[allow(clippy::print_stdout)]
        {
            print!("{}", print.as_ref())
        }
    }
}
