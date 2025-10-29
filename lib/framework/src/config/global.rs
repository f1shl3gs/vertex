use std::fs::DirBuilder;
use std::path::PathBuf;

use log_schema::LogSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{ProxyConfig, skip_serializing_if_default};
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
    /// The directory used for persisting Vertex state data.
    ///
    /// This is the directory where Vertex will store any state data, such as disk buffers, file
    /// checkpoints, and more.
    ///
    /// Vertex must have write permissions to this directory.
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
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
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

    /// Merge a second global configuration into self, and return the new merged
    /// data.
    ///
    /// # Errors
    ///
    /// Returns a list of textual errors if there is a merge conflict between
    /// the two global configs.
    pub fn merge(&mut self, other: GlobalOptions) -> Result<(), Vec<String>> {
        #[inline]
        fn conflicts<T: PartialEq>(a: Option<&T>, b: Option<&T>) -> bool {
            matches!((a, b), (Some(a), Some(b)) if a != b)
        }

        let mut errs = Vec::new();

        if conflicts(self.proxy.http.as_ref(), other.proxy.http.as_ref()) {
            errs.push("conflicting values for 'proxy.http' found".to_string());
        }
        if conflicts(self.proxy.https.as_ref(), other.proxy.https.as_ref()) {
            errs.push("conflicting values for 'proxy.https' found".to_string());
        }
        if !self.proxy.no_proxy.is_empty() && !other.proxy.no_proxy.is_empty() {
            errs.push("conflicting values for 'proxy.no_proxy' found".to_string());
        }

        /*
        if conflicts(self.timezone.as_ref(), other.timezone.as_ref()) {
            errs.push("conflicting values for 'timezone' found".to_string());
        }
        */

        if self.data_dir.is_none() || self.data_dir == default_data_dir() {
            self.data_dir = other.data_dir;
        } else if other.data_dir != default_data_dir() && self.data_dir != other.data_dir {
            // if two configs both set 'data_dir' and have conflicting values,
            // we consider this an error
            errs.push("conflicting values for 'data_dir' found".to_string());
        }

        // If the user has multiple config files, we must *merge* log schemas
        // until we meet a conflict, then we are allowed to error
        let mut log_schema = self.log_schema.clone();
        if let Err(partial) = log_schema.merge(&other.log_schema) {
            errs.extend(partial);
        }

        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

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

    fn merge<P: Debug, T>(
        af: Option<P>,
        bf: Option<P>,
        set: impl Fn(&mut GlobalOptions, Option<P>),
        result: impl Fn(GlobalOptions) -> T,
    ) -> Result<T, Vec<String>> {
        let mut a = GlobalOptions::default();
        let mut b = GlobalOptions::default();
        set(&mut a, af);
        set(&mut b, bf);

        a.merge(b)?;

        Ok(result(a))
    }

    #[test]
    fn merge_data_dir() {
        let merge = |a, b| {
            merge(
                a,
                b,
                |g, f| g.data_dir = f.map(PathBuf::from),
                |g| g.data_dir.map(|p| p.to_string_lossy().to_string()),
            )
        };

        assert_eq!(merge(None, None), Ok(Some("/var/lib/vertex".into())));
        assert_eq!(merge(Some("/test1"), None), Ok(Some("/test1".into())));
        assert_eq!(merge(None, Some("/test2")), Ok(Some("/test2".into())));
        assert_eq!(merge(Some("/foo"), Some("/foo")), Ok(Some("/foo".into())));
        assert_eq!(
            merge(Some("/foo"), Some("/bar")),
            Err(vec!["conflicting values for 'data_dir' found".into()])
        )
    }

    #[test]
    fn merge_proxy_http() {
        let merge = |a, b| {
            merge(
                a,
                b,
                |g, f| g.proxy.http = f.map(|s: &str| s.to_string()),
                |g| g.proxy.http,
            )
        };

        assert_eq!(merge(None, None), Ok(None));
        assert_eq!(merge(Some("test1"), None), Ok(Some("test1".into())));
    }
}
