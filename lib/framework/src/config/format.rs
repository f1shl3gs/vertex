use serde::de;
use std::path::Path;

/// The format used to represent the configuration data.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub enum Format {
    JSON,
    #[default]
    YAML,
}

impl Format {
    pub fn from_path<T: AsRef<Path>>(path: T) -> Result<Self, T> {
        match path.as_ref().extension().and_then(|ext| ext.to_str()) {
            Some("yaml") | Some("yml") => Ok(Format::YAML),
            Some("json") => Ok(Format::JSON),
            _ => Err(path),
        }
    }
}

pub type FormatHint = Option<Format>;

/// Parse the string represented in the specified format.
/// If the format is unknown - fallback to the default format and attempt
/// parsing using that.
pub fn deserialize<T>(content: &str, format: FormatHint) -> Result<T, Vec<String>>
where
    T: de::DeserializeOwned,
{
    match format.unwrap_or_default() {
        Format::YAML => serde_yaml::from_str(content).map_err(|e| vec![e.to_string()]),
        Format::JSON => serde_json::from_str(content).map_err(|e| vec![e.to_string()]),
    }
}
