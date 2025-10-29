use std::path::PathBuf;

use super::{Format, FormatHint};

#[derive(Debug, Clone, Ord, PartialOrd, PartialEq, Eq)]
pub enum ConfigPath {
    File(PathBuf, FormatHint),
    Dir(PathBuf),
}

impl<'a> From<&'a ConfigPath> for &'a PathBuf {
    fn from(path: &'a ConfigPath) -> Self {
        match path {
            ConfigPath::File(path, _) => path,
            ConfigPath::Dir(path) => path,
        }
    }
}

/// Expand a list of paths (potentially containing glob patterns) into real
/// config paths, replacing it with the default paths when empty.
pub fn process_paths(paths: &[PathBuf]) -> Option<Vec<ConfigPath>> {
    let mut output = Vec::with_capacity(paths.len());

    for path in paths {
        let matches = match glob::glob(path.to_string_lossy().as_ref()) {
            Ok(matched) => matched.filter_map(Result::ok).collect::<Vec<_>>(),
            Err(err) => {
                error!(
                    message = "failed to read glob pattern",
                    pattern = ?path,
                    ?err
                );

                return None;
            }
        };

        if matches.is_empty() {
            error!(message = "config file not found in path", ?path);

            std::process::exit(exitcode::CONFIG);
        }

        for matched in matches {
            if matched.is_dir() {
                output.push(ConfigPath::Dir(matched));
            } else if matched.is_file() {
                let hint = Format::from_path(&matched).ok();
                output.push(ConfigPath::File(matched, hint));
            }
        }
    }

    output.dedup();

    Some(output)
}
