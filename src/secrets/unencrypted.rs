use std::collections::HashMap;

use configurable::configurable_component;
use framework::secret::{Error, SecretStore};

/// Configuration for unencrypted store, which is great for tests
#[configurable_component(secret, name = "unencrypted")]
struct Config {
    /// Key/Value pairs to replace all secrets.
    data: HashMap<String, String>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "unencrypted")]
impl SecretStore for Config {
    async fn retrieve(&self, keys: Vec<String>) -> Result<HashMap<String, String>, Error> {
        let mut data = HashMap::with_capacity(keys.len());
        for key in keys {
            match self.data.get(key.as_str()) {
                Some(value) => {
                    data.insert(key, value.to_string());
                }
                None => return Err(Error::NotFound(key)),
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
