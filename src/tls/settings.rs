use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const PEM_START_MARKER: &str = "-----BEGIN ";

#[cfg(test)]
pub const TEST_PEM_CA_PATH: &str = "tests/data/Vector_CA.crt";
#[cfg(test)]
pub const TEST_PEM_CRT_PATH: &str = "tests/data/localhost.crt";
#[cfg(test)]
pub const TEST_PEM_KEY_PATH: &str = "tests/data/localhost.key";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TLSOptions {
    pub verify_certificate: Option<bool>,
    pub verify_hostname: Option<bool>,
    pub ca_file: Option<PathBuf>,
    pub crt_file: Option<PathBuf>,
    pub key_file: Option<PathBuf>,
    pub key_pass: Option<String>,
}

impl TLSOptions {
    #[cfg(test)]
    pub fn test_options() -> Self {
        Self {
            ca_file: Some(TEST_PEM_CA_PATH.into()),
            crt_file: Some(TEST_PEM_CRT_PATH.into()),
            key_file: Some(TEST_PEM_KEY_PATH.into()),
            ..Self::default()
        }
    }

    fn load_authorities(&self) {
        todo!()
    }
}


#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TLSConfig {
    pub options: Option<TLSOptions>,
}