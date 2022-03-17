use std::fs::DirBuilder;
use std::path::PathBuf;

use log_schema::LogSchema;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

use crate::config::{skip_serializing_if_default, ProxyConfig};
use crate::timezone;

#[derive(Debug, Snafu)]
pub enum DataDirError {
    #[snafu(display("data_dir option required, but not given here or globally"))]
    MissingDataDir,
    #[snafu(display("data_dir {:?} does not exist", path))]
    NotExist { path: PathBuf },

    #[snafu(display("data_dir {:?} is not writable", path))]
    NotWritable { path: PathBuf },

    #[snafu(display(
        "could not create sub dir {:?} inside of data dir {:?}: {}",
        subdir,
        data_dir,
        source
    ))]
    CouldNotCreate {
        subdir: PathBuf,
        data_dir: PathBuf,
        source: std::io::Error,
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
            return Err(DataDirError::NotExist { path: dir });
        }

        let readonly = std::fs::metadata(&dir)
            .map(|meta| meta.permissions().readonly())
            .unwrap_or(true);
        if readonly {
            return Err(DataDirError::NotWritable { path: dir });
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
            .with_context(|_kind| CouldNotCreateSnafu {
                subdir,
                data_dir: root,
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
        assert_eq!(global.data_dir, default_data_dir())
    }
}
