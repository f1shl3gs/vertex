use std::fs::DirBuilder;
use std::path::PathBuf;

use log_schema::LogSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::{skip_serializing_if_default, ProxyConfig};
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

/// Global configuration options.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct GlobalOptions {
    /// The directory used for persisting Vector state data.
    ///
    /// This is the directory where Vector will store any state data, such as disk buffers, file
    /// checkpoints, and more.
    ///
    /// Vector must have write permissions to this directory.
    #[serde(default = "default_data_dir")]
    pub data_dir: Option<PathBuf>,

    /// The name of the timezone to apply to timestamp conversions that do not contain an explicit timezone.
    ///
    /// The timezone name may be any name in the [TZ database][tzdb], or `local` to indicate system
    /// local time.
    ///
    /// [tzdb]: https://en.wikipedia.org/wiki/List_of_tz_database_time_zones
    #[serde(default = "default_timezone")]
    pub timezone: timezone::TimeZone,

    #[serde(skip_serializing_if = "skip_serializing_if_default")]
    pub proxy: ProxyConfig,

    #[serde(skip_serializing_if = "skip_serializing_if_default")]
    pub log_schema: LogSchema,

    /// Controls how acknowledgements are handled for all sinks by default.
    ///
    /// See [End-to-end Acknowledgements][e2e_acks] for more information on how Vector handles event
    /// acknowledgement.
    ///
    /// [e2e_acks]: https://vector.dev/docs/about/under-the-hood/architecture/end-to-end-acknowledgements/
    #[serde(
        default,
        skip_serializing_if = "crate::config::skip_serializing_if_default"
    )]
    pub acknowledgements: bool,
}

impl Default for GlobalOptions {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            timezone: default_timezone(),
            proxy: Default::default(),
            log_schema: Default::default(),
            acknowledgements: false,
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

        let input = "
timezone: CET
";
        let global: GlobalOptions = serde_yaml::from_str(input).unwrap();
        assert_eq!(global.data_dir, default_data_dir());
    }
}
