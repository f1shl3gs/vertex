use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EncodingConfig {
    pub charset: &'static encoding_rs::Encoding,
}
