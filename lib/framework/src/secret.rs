use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("secret key {0:?} was not found in the store")]
    NotFound(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[async_trait::async_trait]
#[typetag::serde(tag = "type")]
pub trait SecretStore: Debug + Send + Sync {
    async fn retrieve(&self, keys: Vec<String>) -> Result<HashMap<String, String>, Error>;
}
