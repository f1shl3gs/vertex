use std::path::Path;

pub type FormatHint = Option<Format>;

/// The format used to represent the configuration data.
/// YAML for human, JSON for program that all
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub enum Format {
    JSON,
    #[default]
    YAML,
}

impl Format {
    pub fn from_path<T: AsRef<Path>>(path: T) -> Result<Self, T> {
        let Some(ext) = path.as_ref().extension() else {
            return Err(path);
        };

        if ext == "json" {
            Ok(Format::JSON)
        } else if ext == "yaml" || ext == "yml" {
            Ok(Format::YAML)
        } else {
            Err(path)
        }
    }

    /// Parse the string represented in the specified format.
    /// If the format is unknown - fallback to the default format and attempt
    /// parsing using that.
    pub fn deserialize<T: serde::de::DeserializeOwned>(&self, content: &str) -> Result<T, String> {
        match self {
            Format::JSON => serde_json::from_str(content).map_err(|err| err.to_string()),
            Format::YAML => serde_json::from_str(content).map_err(|err| err.to_string()),
        }
    }
}
