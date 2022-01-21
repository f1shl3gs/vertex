use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HttpSourceAuthConfig {
    pub username: String,
    pub password: String,
}

impl HttpSourceAuthConfig {}
