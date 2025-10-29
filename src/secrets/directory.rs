use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;

use configurable::configurable_component;
use framework::secret::{Error, SecretStore};

#[configurable_component(secret, name = "directory")]
struct Config {
    /// Directory path to read secrets from
    path: PathBuf,

    /// Remove trailing whitespace from file contents
    #[serde(default)]
    trim_end: bool,
}

#[async_trait::async_trait]
#[typetag::serde(name = "directory")]
impl SecretStore for Config {
    async fn retrieve(&self, keys: Vec<String>) -> Result<HashMap<String, String>, Error> {
        let mut data = HashMap::new();

        for key in keys {
            let path = self.path.join(&key);

            match std::fs::read_to_string(&path) {
                Ok(mut content) => {
                    if self.trim_end {
                        let len = content.trim_end().len();
                        content.truncate(len);
                    }

                    data.insert(key, content);
                }

                Err(err) => {
                    if err.kind() == ErrorKind::NotFound {
                        return Err(Error::NotFound(key));
                    }

                    return Err(Error::Io(err));
                }
            }
        }

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
