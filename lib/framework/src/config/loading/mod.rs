mod config;
mod env;
mod graph;
mod secret;

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;

use super::{Config, ConfigPath, Format, FormatHint};
use crate::signal::SignalHandler;
pub use config::{Builder, ConfigLoader};

/// Since Vertex will store all the config in the memory, so a limitation is
/// very necessary for safety. If a middle type is introduced we can save
/// some memory.
///
/// Default to 128Mib, which should be large enough.
const CONFIG_CACHE_LIMIT: usize = 128 * 1024 * 1024;

pub trait Loader {
    type Output;
    type Item: serde::de::DeserializeOwned;

    fn prepare<'a>(&mut self, input: &'a str) -> Result<Cow<'a, str>, Vec<String>> {
        Ok(Cow::Borrowed(input))
    }

    fn merge(&mut self, value: Self::Item) -> Result<(), Vec<String>>;

    fn build(self) -> Result<Self::Output, Vec<String>>;
}

fn process<L: Loader>(
    configs: &mut IndexMap<(PathBuf, Format), String>,
    mut loader: L,
) -> Result<L::Output, Vec<String>> {
    let mut errs = Vec::new();

    for ((path, format), content) in configs {
        match loader.prepare(content) {
            Ok(Cow::Owned(new)) => *content = new,
            Err(partial) => errs.extend(partial),
            _ => {}
        }

        // deserialize and merge
        match format {
            Format::JSON => match serde_json::from_str::<L::Item>(content) {
                Ok(item) => {
                    if let Err(partial) = loader.merge(item) {
                        errs.extend(partial);
                    }
                }
                Err(err) => {
                    errs.push(format!("deserialize {path:?} failed, {err}"));
                }
            },
            Format::YAML => {
                // multiple documentations
                for doc in serde_yaml::Deserializer::from_str(content) {
                    use serde::Deserialize;

                    match L::Item::deserialize(doc) {
                        Ok(item) => {
                            if let Err(partial) = loader.merge(item) {
                                errs.extend(partial);
                            }
                        }
                        Err(err) => {
                            errs.push(format!("deserialize {path:?} failed, {err}"));
                        }
                    }
                }
            }
        }
    }

    if !errs.is_empty() {
        return Err(errs);
    }

    match loader.build() {
        Ok(output) => Ok(output),
        Err(partial) => {
            errs.extend(partial);
            Err(errs)
        }
    }
}

pub fn load_builder_from_paths(paths: &[ConfigPath]) -> Result<Builder, Vec<String>> {
    let mut configs = load_configs(paths)?;
    let mut errs = Vec::new();

    let vars = env::loading();
    for (_, content) in &mut configs {
        match env::interpolate(content, &vars) {
            Ok(Cow::Owned(new)) => *content = new,
            Err(partial) => errs.extend(partial),
            _ => {}
        }
    }

    if !errs.is_empty() {
        return Err(errs);
    }

    process(&mut configs, ConfigLoader::new(Default::default()))
}

#[cfg(any(test, feature = "test-util"))]
pub fn load_from_str(content: &str, format: Format) -> Result<Config, Vec<String>> {
    let mut configs = IndexMap::new();
    configs.insert((PathBuf::new(), format), content.to_string());

    let mut errs = Vec::new();
    let vars = env::loading();
    for (_, content) in &mut configs {
        match env::interpolate(content, &vars) {
            Ok(Cow::Owned(new)) => *content = new,
            Err(partial) => errs.extend(partial),
            _ => {}
        }
    }

    if !errs.is_empty() {
        return Err(errs);
    }

    process(&mut configs, ConfigLoader::new(Default::default()))?.compile()
}

/// Loads a configuration from paths, If a provider is present in the builder,
/// the config is used as bootstrapping for a remote source. Otherwise, provider
/// instantiation is skipped.
pub async fn load_from_paths_with_provider_and_secrets(
    paths: &[ConfigPath],
    signal: &mut SignalHandler,
) -> Result<Config, Vec<String>> {
    let mut configs = load_configs(paths)?;

    let mut errs = Vec::new();

    // handle envs
    let vars = env::loading();
    for (_, content) in &mut configs {
        match env::interpolate(content, &vars) {
            Ok(new) => {
                if let Cow::Owned(new) = new {
                    *content = new;
                }
            }
            Err(partial) => {
                errs.extend(partial);
            }
        }
    }

    if !errs.is_empty() {
        return Err(errs);
    }

    let secret_loader = process(&mut configs, secret::SecretLoader::default())?;
    let secrets = secret_loader.retrieve().await?;

    let mut builder = process(&mut configs, config::ConfigLoader::new(secrets))?;
    if let Some(provider) = &mut builder.provider {
        info!(message = "Provider configured", provider = ?provider.component_name());
        builder = provider.build(signal).await?;
    }

    match builder.compile() {
        Ok(config) => {
            for warning in config.warnings() {
                warn!(message = warning);
            }

            Ok(config)
        }
        Err(partial) => {
            errs.extend(partial);
            Err(errs)
        }
    }
}

fn load_configs(paths: &[ConfigPath]) -> Result<IndexMap<(PathBuf, Format), String>, Vec<String>> {
    let mut configs = IndexMap::<(PathBuf, Format), String>::with_capacity(paths.len());
    let mut errs = Vec::new();
    let mut size = 0usize;

    for path in paths {
        match path {
            ConfigPath::File(path, _) => {
                let Ok(format) = Format::from_path(&path) else {
                    continue;
                };

                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        size += content.len();
                        if size > CONFIG_CACHE_LIMIT {
                            errs.push(format!(
                                "config cache {size} exceed the memory limitation {CONFIG_CACHE_LIMIT}",
                            ));

                            return Err(errs);
                        }

                        configs.insert((path.to_path_buf(), format), content);
                    }
                    Err(err) => {
                        errs.push(format!("reading file {path:?} failed, {err}"));
                    }
                }
            }
            ConfigPath::Dir(dir) => {
                load_directory(dir, &mut configs, &mut errs);
            }
        }
    }

    if errs.is_empty() {
        Ok(configs)
    } else {
        Err(errs)
    }
}

fn load_directory(
    path: &Path,
    inputs: &mut IndexMap<(PathBuf, Format), String>,
    errs: &mut Vec<String>,
) {
    let dirs = match path.read_dir() {
        Ok(dirs) => dirs,
        Err(err) => {
            errs.push(format!("reading directory {path:?} failed, {err}"));
            return;
        }
    };

    for result in dirs {
        match result {
            Ok(entry) => {
                let path = entry.path();

                if path.is_dir() {
                    load_directory(&path, inputs, errs);
                    continue;
                } else if !path.is_file() {
                    continue;
                }

                let Ok(format) = Format::from_path(&path) else {
                    continue;
                };

                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        inputs.insert((path, format), content);
                    }
                    Err(err) => {
                        errs.push(format!("reading file {path:?} failed, {err}"));
                    }
                }
            }
            Err(err) => {
                errs.push(format!("iterating directory {path:?} failed, {err}"));
            }
        }
    }
}

pub fn load<R: std::io::Read>(
    mut input: R,
    hint: FormatHint,
) -> Result<(Builder, Vec<String>), Vec<String>> {
    let mut content = String::new();
    input
        .read_to_string(&mut content)
        .map_err(|err| vec![err.to_string()])?;

    let envs = env::loading();
    let interpolated = env::interpolate(content.as_str(), &envs)?;

    let format = match hint {
        Some(format) => format,
        None => {
            if interpolated.starts_with("{") {
                Format::JSON
            } else {
                Format::YAML
            }
        }
    };

    format.deserialize(&interpolated).map_err(|err| vec![err])
}
