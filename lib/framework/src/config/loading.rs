use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use tracing::error;

use super::validation;
use crate::config::{format, Builder, Config, ConfigPath, Format, FormatHint};
use crate::signal;

pub static CONFIG_PATHS: Lazy<Mutex<Vec<ConfigPath>>> = Lazy::new(Mutex::default);

/// Loads a configuration from path. If a provider is present in the builder, the
/// config is used as bootstrapping for a remote source. Otherwise, provider
/// instantiation is skipped.
pub async fn load_from_paths_with_provider(
    paths: &[ConfigPath],
    signal_handler: &mut signal::SignalHandler,
) -> Result<Config, Vec<String>> {
    let (mut builder, load_warnings) = load_builder_from_paths(paths)?;
    validation::check_provider(&builder)?;
    signal_handler.clear();

    // If there's a provider, overwrite the existing config builder with
    // the remote variant
    if let Some(mut provider) = builder.provider {
        builder = provider.build(signal_handler).await?;
        info!(
            message = "Provider configured",
            provider = ?provider.component_name()
        );
    }

    let (config, build_warnings) = builder.build_with_warnings()?;

    for warning in load_warnings.into_iter().chain(build_warnings) {
        warn!("{}", warning);
    }

    Ok(config)
}

pub fn load_builder_from_paths(
    paths: &[ConfigPath],
) -> Result<(Builder, Vec<String>), Vec<String>> {
    let mut inputs = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match path {
            ConfigPath::File(path, format) => {
                if let Some(file) = open_config(path) {
                    inputs.push((file, format.or_else(move || Format::from_path(&path).ok())));
                } else {
                    errors.push(format!("Config file not found in path: {:?}", path))
                };
            }

            ConfigPath::Dir(path) => match path.read_dir() {
                Ok(readdir) => {
                    for res in readdir {
                        match res {
                            Ok(ent) => {
                                if let Some(file) = open_config(&ent.path()) {
                                    // skip files who's format is unknown
                                    if let Ok(format) = Format::from_path(ent.path()) {
                                        inputs.push((file, Some(format)));
                                    }
                                }
                            }
                            Err(err) => errors.push(format!(
                                "Could not read file in config dir: {:?}, {}",
                                path, err
                            )),
                        }
                    }
                }

                Err(err) => {
                    errors.push(format!("Could not read config dir: {:?}, {}", path, err));
                }
            },
        }
    }

    if errors.is_empty() {
        load_from_inputs(inputs)
    } else {
        Err(errors)
    }
}

fn open_config(path: &Path) -> Option<File> {
    match File::open(path) {
        Ok(f) => Some(f),
        Err(err) => {
            if let std::io::ErrorKind::NotFound = err.kind() {
                error!(message = "Config file not found in path", ?path);
            } else {
                error!(
                    message = "Error opening config file",
                    %err,
                    ?path
                );
            }

            None
        }
    }
}

fn load_from_inputs(
    inputs: impl IntoIterator<Item = (impl std::io::Read, FormatHint)>,
) -> Result<(Builder, Vec<String>), Vec<String>> {
    let mut builder = Builder::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for (input, format) in inputs {
        if let Err(errs) = load(input, format).and_then(|(n, mut load_warnings)| {
            warnings.append(&mut load_warnings);
            builder.append(n)
        }) {
            // TODO; add back paths
            errors.extend(errs.iter().map(|e| e.to_string()));
        }
    }

    if errors.is_empty() {
        Ok((builder, warnings))
    } else {
        Err(errors)
    }
}

pub fn load(
    mut input: impl std::io::Read,
    format: FormatHint,
) -> Result<(Builder, Vec<String>), Vec<String>> {
    let mut ss = String::new();
    input
        .read_to_string(&mut ss)
        .map_err(|err| vec![err.to_string()])?;

    let mut vars = std::env::vars().collect::<HashMap<_, _>>();
    if !vars.contains_key("HOSTNAME") {
        if let Ok(hostname) = get_hostname() {
            vars.insert("HOSTNAME".into(), hostname);
        }
    }

    let (with_vars, warnings) = interpolate(&ss, &vars);

    format::deserialize(&with_vars, format).map(|builder| (builder, warnings))
}

pub fn load_from_str(content: &str, format: Format) -> Result<Config, Vec<String>> {
    let (builder, load_warnings) =
        load_from_inputs(std::iter::once((content.as_bytes(), Some(format))))?;
    let (config, build_warnings) = builder.build_with_warnings()?;

    load_warnings
        .into_iter()
        .chain(build_warnings)
        .for_each(|warning| {
            warn!("{}", warning);
        });

    Ok(config)
}

pub fn get_hostname() -> std::io::Result<String> {
    Ok(hostname::get()?.to_string_lossy().into())
}

/// (result, warnings)
fn interpolate(input: &str, vars: &HashMap<String, String>) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let re = Regex::new(r"\$\$|\$(\w+)|\$\{(\w+)(?::-([^}]+)?)?\}").unwrap();
    let interpolated = re
        .replace_all(input, |caps: &Captures<'_>| {
            caps.get(1)
                .or_else(|| caps.get(2))
                .map(|m| m.as_str())
                .map(|name| {
                    vars.get(name).map(|val| val.as_str()).unwrap_or_else(|| {
                        caps.get(3).map(|m| m.as_str()).unwrap_or_else(|| {
                            warnings.push(format!("Unknown env var in config. name = {:?}", name));
                            ""
                        })
                    })
                })
                .unwrap_or("$")
                .to_string()
        })
        .into_owned();

    (interpolated, warnings)
}

/// Merge the paths coming from different cli flags with different formats
/// into a unified list of paths with formats
pub fn merge_path_lists(
    list: Vec<(&[PathBuf], FormatHint)>,
) -> impl Iterator<Item = (PathBuf, FormatHint)> + '_ {
    list.into_iter()
        .flat_map(|(paths, format)| paths.iter().cloned().map(move |path| (path, format)))
}

#[cfg(unix)]
fn default_config_paths() -> Vec<ConfigPath> {
    vec![ConfigPath::File(
        "/etc/vertex/vertex.yaml".into(),
        Some(Format::YAML),
    )]
}

#[cfg(not(unix))]
fn default_config_paths() -> Vec<ConfigPath> {
    vec![]
}

pub fn process_paths(paths: &[ConfigPath]) -> Option<Vec<ConfigPath>> {
    let default_paths = default_config_paths();
    let starting_paths = if !paths.is_empty() {
        paths
    } else {
        &default_paths
    };

    let mut paths = Vec::new();
    for path in starting_paths {
        let pattern: &PathBuf = path.into();

        let matches: Vec<PathBuf> = match glob::glob(pattern.to_str().expect("No ability to glob"))
        {
            Ok(gp) => gp.flat_map(Result::ok).collect(),
            Err(err) => {
                error!(
                    message = "Failed to read glob pattern",
                    path = ?pattern,
                    ?err
                );

                return None;
            }
        };

        if matches.is_empty() {
            error!(
                message = "Config file not found in path",
                path = ?pattern
            );

            std::process::exit(exitcode::CONFIG);
        }

        match path {
            ConfigPath::File(_, format) => {
                for path in matches {
                    paths.push(ConfigPath::File(path, *format));
                }
            }

            ConfigPath::Dir(_) => {
                for path in matches {
                    paths.push(ConfigPath::Dir(path));
                }
            }
        }
    }

    paths.sort();
    paths.dedup();

    // Ignore poison error and let the current main thread continue
    // running to do the cleanup
    std::mem::drop(CONFIG_PATHS.lock().map(|mut guard| *guard = paths.clone()));

    Some(paths)
}

#[cfg(test)]
mod tests {
    // Since framework is created extensions, sources, transforms and sinks are not registered
    // anymore, so this tests will fail
    use super::*;

    const INPUT: &str = r#"
global:
  data_dir: ./temp

health_checks:
  enabled: false

extensions:
  pprof:
    type: pprof
    listen: 0.0.0.0:9000

sources:
  kmsg:
    type: kmsg
  node:
    type: node_metrics
    interval: 15s
  selfstat:
    type: selfstat
  generator:
    type: generator
  journald:
    type: journald
    units: []
    excludes: []

transforms:
  add_extra_tags:
    type: rewrite
    inputs:
      - generator
      # - ntp
    operations:
      - type: set
        key: hostname
        value: ${HOSTNAME}

sinks:
  blackhole:
    type: blackhole
    inputs:
      - journald
  stdout:
    type: blackhole
    inputs:
      - kmsg
  prom:
    type: prometheus_exporter
    inputs:
      - add_extra_tags
      - node
      - selfstat
    listen: 127.0.0.1:9101

        "#;

    #[test]
    #[ignore]
    fn test_load_from_str() {
        let config = load_from_str(INPUT, Format::YAML).unwrap();
        assert_eq!(config.sources.len(), 5);
    }

    #[test]
    #[ignore]
    fn test_load() {
        let cursor = std::io::Cursor::new(INPUT.to_string());
        let reader = std::io::BufReader::new(cursor);

        let (builder, warnings) = load(reader, format::FormatHint::Some(Format::YAML)).unwrap();
        assert_eq!(warnings.len(), 0);
        assert_eq!(builder.sources.len(), 5);
        assert_eq!(builder.transforms.len(), 1);
        assert_eq!(builder.sinks.len(), 3);
        assert_eq!(builder.extensions.len(), 1);
    }
}
