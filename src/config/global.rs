use std::fs::DirBuilder;
use std::path::PathBuf;
use snafu::{ResultExt, Snafu};
use crate::timezone;
use serde::{Deserialize, Serialize};

#[derive(Debug, Snafu)]
pub enum DataDirError {
    #[snafu(display("data_dir option required, but not given here or globally"))]
    MissingDataDir,
    #[snafu(display("data_dir {:?} does not exist", path))]
    NotExist {
        path: PathBuf
    },

    #[snafu(display("data_dir {:?} is not writable", path))]
    NotWritable {
        path: PathBuf
    },

    #[snafu(display(
    "could not create sub dir {:?} inside of data dir {:?}: {}",
    subdir, data_dir, source
    ))]
    CouldNotCreate {
        subdir: PathBuf,
        data_dir: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct GlobalOptions {
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    #[serde(default = "default_timezone")]
    pub timezone: timezone::TimeZone,
}

pub fn default_data_dir() -> PathBuf {
    PathBuf::from("/var/lib/vertex")
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
    pub fn validate_data_dir(
        &self,
    ) -> Result<PathBuf, DataDirError> {
        let dir = self.data_dir
            .clone();

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
    pub fn make_subdir(
        &self,
        subdir: &str,
    ) -> Result<PathBuf, DataDirError> {
        let root = self.validate_data_dir()?;
        let subdir = root.clone().join(subdir);
        let rt = subdir.clone();

        DirBuilder::new()
            .recursive(true)
            .create(&subdir)
            .with_context(|| CouldNotCreate { subdir, data_dir: root })?;

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
        assert_eq!(global.data_dir, PathBuf::from("foo"));

        let input = "
timezone: CET
";
        let global: GlobalOptions = serde_yaml::from_str(input).unwrap();
        assert_eq!(global.data_dir, default_data_dir())
    }
}