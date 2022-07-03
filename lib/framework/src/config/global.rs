use std::fs::DirBuilder;
use std::path::PathBuf;
use std::time::Duration;

use log_schema::LogSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::{
    default_interval, deserialize_duration, serialize_duration, skip_serializing_if_default,
    ProxyConfig,
};
use crate::timezone;

#[derive(Debug, Error)]
pub enum DataDirError {
    #[error("data_dir option required, but not given here or globally")]
    MissingDataDir,
    #[error("data_dir {0:?} does not exist")]
    NotExist(PathBuf),

    #[error("data_dir {0:?} is not writable")]
    NotWritable(PathBuf),

    #[error("could not create sub dir {subdir:?} inside of data dir {data_dir:?}: {err}")]
    CouldNotCreate {
        subdir: PathBuf,
        data_dir: PathBuf,
        err: std::io::Error,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct GlobalOptions {
    #[serde(default = "default_data_dir")]
    pub data_dir: Option<PathBuf>,
    #[serde(default = "default_timezone")]
    pub timezone: timezone::TimeZone,
    #[serde(skip_serializing_if = "skip_serializing_if_default")]
    pub proxy: ProxyConfig,
    #[serde(skip_serializing_if = "skip_serializing_if_default")]
    pub log_schema: LogSchema,
    #[serde(
        default,
        skip_serializing_if = "crate::config::skip_serializing_if_default"
    )]
    pub acknowledgements: bool,

    // How often the sources should report metrics.
    //
    // NB: not all source need this, they might report events
    // as soon as possible once they receive any.
    #[serde(default = "default_interval")]
    #[serde(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub interval: Duration,
}

impl Default for GlobalOptions {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            timezone: default_timezone(),
            proxy: Default::default(),
            log_schema: Default::default(),
            acknowledgements: false,
            interval: default_interval(),
        }
    }
}

pub fn default_data_dir() -> Option<PathBuf> {
    Some(PathBuf::from("/var/lib/vertex"))
}

fn default_timezone() -> timezone::TimeZone {
    Default::default()
}

impl GlobalOptions {
    /// Resolve the `data_dir` option in either the global or local config, and
    /// valid that it exists and is writable
    ///
    /// # Errors
    ///
    /// Function will error if it is unable to make data directory
    pub fn validate_data_dir(&self) -> Result<PathBuf, DataDirError> {
        let data_dir = self.data_dir.clone();
        let dir = data_dir.ok_or(DataDirError::MissingDataDir)?;

        if !dir.exists() {
            return Err(DataDirError::NotExist(dir));
        }

        let readonly = std::fs::metadata(&dir)
            .map(|meta| meta.permissions().readonly())
            .unwrap_or(true);
        if readonly {
            return Err(DataDirError::NotWritable(dir));
        }
        Ok(dir)
    }

    /// Resolve the `data_dir` option using `resolve_and_validate_data_dir` and
    /// the ensure a named subdirectory exists
    ///
    /// # Errors
    ///
    /// Function will error if it is unable to make data subdirectory
    pub fn make_subdir(&self, subdir: &str) -> Result<PathBuf, DataDirError> {
        let root = self.validate_data_dir()?;
        let subdir = root.join(subdir);
        let rt = subdir.clone();

        DirBuilder::new()
            .recursive(true)
            .create(&subdir)
            .map_err(|err| DataDirError::CouldNotCreate {
                subdir,
                data_dir: root,
                err,
            })?;

        Ok(rt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let input = r#"
data_dir: foo
"#;
        let global: GlobalOptions = serde_yaml::from_str(input).unwrap();
        assert_eq!(global.data_dir.unwrap(), PathBuf::from("foo"));
        assert_eq!(global.interval, default_interval());

        let input = "
timezone: CET
interval: 10s
";
        let global: GlobalOptions = serde_yaml::from_str(input).unwrap();
        assert_eq!(global.data_dir, default_data_dir());
        assert_eq!(global.interval, Duration::from_secs(10));
    }
}
