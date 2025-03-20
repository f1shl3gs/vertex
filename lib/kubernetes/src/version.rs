use serde::Deserialize;
use std::num::ParseFloatError;

/// Info contains versioning information. how we'll want to distribute that information
#[derive(Deserialize, Debug)]
pub struct Version {
    /// Major version of the ApiServer
    pub major: String,

    /// Minor version of the ApiServer
    pub minor: String,

    pub platform: String,
}

impl Version {
    #[inline]
    pub fn number(&self) -> Result<f64, ParseFloatError> {
        format!("{}.{}", self.major, self.minor).parse()
    }
}
