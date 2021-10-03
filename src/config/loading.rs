use std::collections::HashMap;
use regex::{Captures, Regex};
use std::fs::File;

use crate::config::{FormatHint, Config, Builder, Format, format, ConfigPath};
use crate::signal;
use std::path::{Path};

/// Loads a configuration from path. If a provider is present in the builder, the
/// config is used as bootstrapping for a remote source. Otherwise, provider
/// instantiation is skipped.
pub async fn load_from_paths_with_provider(
    paths: &[ConfigPath],
    signal_handler: &mut signal::SignalHandler,
) -> Result<Config, Vec<String>> {
    let (mut builder, load_warnings) = load_builder_from_paths(paths)?;
    signal_handler.clear();

    if let Some(mut provider) = builder.provider {
        builder = provider.build(signal_handler).await?
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
                    inputs.push(
                        (file, format.or_else(move || Format::from_path(&path).ok()))
                    );
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
                            Err(err) => {
                                errors.push(
                                    format!("Could not read file in config dir: {:?}, {}", path, err)
                                )
                            }
                        }
                    }
                }

                Err(err) => {
                    errors.push(
                        format!("Could not read config dir: {:?}, {}", path, err)
                    );
                }
            }
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
                None
            } else {
                None
            }
        }
    }
}

fn load_from_inputs(
    inputs: impl IntoIterator<Item=(impl std::io::Read, FormatHint)>,
) -> Result<(Builder, Vec<String>), Vec<String>> {
    let mut builder = Builder::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for (input, format) in inputs {
        if let Err(errs) = load(input, format)
            .and_then(|(n, mut load_warnings)| {
                warnings.append(&mut load_warnings);
                builder.append(n)
            })
        {
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
    input.read_to_string(&mut ss)
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

fn load_from_str(content: String) -> Result<(Config, Vec<String>), Vec<String>> {
    let mut vars = std::env::vars().collect::<HashMap<_, _>>();
    if !vars.contains_key("HOSTNAME") {
        if let Ok(hostname) = get_hostname() {
            vars.insert("HOSTNAME".into(), hostname);
        }
    }

    let (with_vars, warnings) = interpolate(&content, &vars);

    format::deserialize(&with_vars, Some(Format::YAML))
        .map(|config| (config, warnings))
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
        }).into_owned();

    (interpolated, warnings)
}
