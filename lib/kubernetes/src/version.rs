use serde::Deserialize;

/// Info contains versioning information. how we'll want to distribute that information
#[derive(Deserialize, Debug)]
pub struct Version {
    /// Major version of the ApiServer
    pub major: String,

    /// Minor version of the ApiServer
    pub minor: String,

    pub platform: String,
}
