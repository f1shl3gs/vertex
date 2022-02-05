use headers::{Authorization, HeaderMapExt};
use http::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

use crate::config::GenerateConfig;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HttpSourceAuthConfig {
    pub username: String,
    pub password: String,
}

impl GenerateConfig for HttpSourceAuthConfig {
    fn generate_config() -> String {
        r#"
# The basic authentication user name.
username: username

# The basic authentication password.
password: password
        "#
        .into()
    }
}

impl TryFrom<Option<&HttpSourceAuthConfig>> for HttpSourceAuth {
    type Error = String;

    fn try_from(config: Option<&HttpSourceAuthConfig>) -> Result<Self, Self::Error> {
        match config {
            Some(config) => {
                let mut headers = HeaderMap::new();
                let token = Authorization::basic(&config.username, &config.password);
                headers.typed_insert(token);

                match headers.get("authorization") {
                    Some(value) => {
                        let token = value
                            .to_str()
                            .map_err(|err| format!("Failed stringify HeaderValue: {:?}", err))?
                            .to_owned();

                        Ok(HttpSourceAuth { token: Some(token) })
                    }
                    None => Ok(HttpSourceAuth { token: None }),
                }
            }

            None => Ok(HttpSourceAuth { token: None }),
        }
    }
}

#[derive(Clone, Debug)]
pub struct HttpSourceAuth {
    pub token: Option<String>,
}

impl HttpSourceAuth {
    pub fn validate(&self, header: Option<&HeaderValue>) -> bool {
        match (&self.token, header) {
            (Some(t1), Some(t2)) => match t2.to_str() {
                Ok(t2) => t1 == t2,
                Err(_err) => false,
            },
            (Some(_), None) => false,
            (None, _) => true,
        }
    }
}
