use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;
use serde::Deserialize;

use crate::secret::{Error, SecretStore};

// The following regex aims to extract a pair of strings, the first being the secret
// store name and the second being the secret key. Here are some matching &
// non-matching examples:
// - "SECRET[store.secret_name]" will match and capture "store" and "secret_name"
// - "SECRET[store.secret.name]" will match and capture "store" and "secret.name"
// - "SECRET[store..secret.name]" will match and capture "store" and ".secret.name"
// - "SECRET[secret_name]" will not match
// - "SECRET[.secret.name]" will not match
pub static COLLECTOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"SECRET\[([[:word:]]+)\.([[:word:].-]+)\]").unwrap());

#[derive(Debug, Deserialize)]
pub struct SecretBuilder {
    #[serde(default)]
    secrets: IndexMap<String, Box<dyn SecretStore>>,
}

#[derive(Default)]
pub struct SecretLoader {
    stores: IndexMap<String, Box<dyn SecretStore>>,
    // key is the store name, values is the keys to retrieve
    keys: HashMap<String, Vec<String>>,
}

impl super::Loader for SecretLoader {
    type Output = SecretLoader;
    type Item = SecretBuilder;

    fn prepare<'a>(&mut self, input: &'a str) -> Result<Cow<'a, str>, Vec<String>> {
        COLLECTOR.captures_iter(input).for_each(|cap| {
            if let (Some(store), Some(key)) = (cap.get(1), cap.get(2)) {
                if let Some(keys) = self.keys.get_mut(store.as_str()) {
                    keys.push(key.as_str().into());
                } else {
                    self.keys
                        .insert(store.as_str().to_string(), vec![key.as_str().into()]);
                }
            }
        });

        Ok(Cow::Borrowed(input))
    }

    fn merge(&mut self, builder: Self::Item) -> Result<(), Vec<String>> {
        if builder.secrets.is_empty() {
            return Ok(());
        }

        let mut errs = Vec::new();

        for (name, store) in builder.secrets {
            if self.stores.contains_key(&name) {
                errs.push(format!("duplicate secret store {name:?}"));

                continue;
            }

            self.stores.insert(name, store);
        }

        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }

    fn build(mut self) -> Result<Self::Output, Vec<String>> {
        let mut errs = Vec::new();

        for (store, keys) in &mut self.keys {
            keys.dedup();

            if !self.stores.contains_key(store) {
                errs.push(format!("secret store {store:?} is used but not defined"))
            }
        }

        if errs.is_empty() { Ok(self) } else { Err(errs) }
    }
}

impl SecretLoader {
    pub async fn retrieve(self) -> Result<HashMap<String, HashMap<String, String>>, Vec<String>> {
        let mut secrets = HashMap::with_capacity(self.stores.len());
        let mut errs = Vec::new();

        for (store, keys) in self.keys {
            match self.stores.get(&store) {
                Some(s) => match s.retrieve(keys).await {
                    Ok(partial) => {
                        let entry: &mut HashMap<String, String> =
                            secrets.entry(store).or_insert_with(Default::default);

                        entry.extend(partial);
                    }
                    Err(err) => {
                        let err = match err {
                            Error::NotFound(key) => {
                                format!("secret {key:?} was not found in {store:?}")
                            }
                            Error::Io(err) => err.to_string(),
                        };

                        errs.push(err);
                    }
                },
                None => {
                    errs.push(format!("secret store {store:?} is not defined"));
                    continue;
                }
            }
        }

        if !errs.is_empty() {
            return Err(errs);
        }

        Ok(secrets)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde::Serialize;

    use super::*;
    use crate::config::Format;
    use crate::config::loading::process;
    use crate::secret::Error;

    #[derive(Debug, Deserialize, Serialize)]
    struct Config {
        data: HashMap<String, String>,
    }

    #[async_trait::async_trait]
    #[typetag::serde(name = "test")]
    impl SecretStore for Config {
        async fn retrieve(&self, keys: Vec<String>) -> Result<HashMap<String, String>, Error> {
            let mut map = HashMap::with_capacity(keys.len());

            for key in keys {
                match self.data.get(key.as_str()) {
                    Some(value) => {
                        map.insert(key, value.to_string());
                    }
                    None => return Err(Error::NotFound(key.to_string())),
                }
            }

            Ok(map)
        }
    }

    #[tokio::test]
    async fn process_and_load() {
        let mut configs = IndexMap::new();
        configs.insert(
            (PathBuf::from("secrets.yaml"), Format::YAML),
            String::from(
                r#"
secrets:
  test:
    type: test
    data:
      foo: bar
"#,
            ),
        );
        configs.insert(
            (PathBuf::from("sources.yaml"), Format::YAML),
            String::from(
                r#"
sources:
  selfstat:
    type: SECRET[test.foo]
"#,
            ),
        );

        let loader = process(&mut configs, SecretLoader::default()).unwrap();
        let secrets = loader.retrieve().await.unwrap();

        assert_eq!(secrets.get("test").unwrap().get("foo").unwrap(), "bar");
    }
}
