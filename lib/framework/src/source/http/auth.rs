use configurable::Configurable;
use headers::{Authorization, HeaderMapExt};
use http::header::AUTHORIZATION;
use http::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

use crate::config::SecretString;

/// HTTP basic auth
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
pub struct HttpSourceAuthConfig {
    /// The basic authentication username.
    #[configurable(required)]
    pub username: String,

    /// The basic authentication password.
    #[configurable(required)]
    pub password: SecretString,
}

impl TryFrom<Option<&HttpSourceAuthConfig>> for HttpSourceAuth {
    type Error = String;

    fn try_from(config: Option<&HttpSourceAuthConfig>) -> Result<Self, Self::Error> {
        match config {
            Some(config) => {
                let mut headers = HeaderMap::new();
                let token = Authorization::basic(&config.username, &config.password);
                headers.typed_insert(token);

                match headers.get(AUTHORIZATION) {
                    Some(value) => {
                        let token = value
                            .to_str()
                            .map_err(|err| format!("Failed stringify HeaderValue: {err:?}"))?
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
