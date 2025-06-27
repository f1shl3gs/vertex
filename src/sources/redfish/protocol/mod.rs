#![allow(dead_code)]

pub mod chassis;
pub mod system;

use serde::{Deserialize, Deserializer};

fn null_to_default<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(d)?;
    let val = opt.unwrap_or_else(String::new);
    Ok(val)
}

/// `Status` is a common structure used in any entity with a status
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Status {
    /// The health state of this resource in the absence of its dependent resources.
    ///
    /// | Valid values | Description |
    /// | ------------ | ----------- |
    /// | Critical | A critical condition requires immediate attention. |
    /// | OK | Normal. |
    /// | Warning | A condition requires attention. |
    ///
    /// This file could be missing, if the state is "Absent"
    pub health: Option<String>,

    // /// The overall health state from the view of this resource.
    // ///
    // /// | Valid values | Description |
    // /// | ------------ | ----------- |
    // /// | Critical | A critical condition requires immediate attention. |
    // /// | OK | Normal. |
    // /// | Warning | A condition requires attention. |
    // pub health_rollup: Option<String>,
    /// The state of the resource
    ///
    /// | Valid values | Description |
    /// | --------- | -------- |
    /// | Absent    | This function or device is not currently present or detected. This resource represents a capability or an available location where a device can be installed. |
    /// | Deferring    | The element does not process any commands but queues new requests. |
    /// | Degraded    | The function or resource is degraded. |
    /// | Disabled    | This function or resource is disabled. |
    /// | Enabled    | This function or resource is enabled. |
    /// | InTest    | This function or resource is undergoing testing or is in the process of capturing information for debugging. |
    /// | Qualified    | The element quality is within the acceptable range of operation. |
    /// | Quiesced    | The element is enabled but only processes a restricted set of commands. |
    /// | StandbyOffline |    This function or resource is enabled but awaits an external action to activate it. |
    /// | StandbySpare |    This function or resource is part of a redundancy set and awaits a failover or other external action to activate it. |
    /// | Starting |    This function or resource is starting. |
    /// | UnavailableOffline |    This function or resource is present but cannot be used. |
    /// | Updating | The element is updating and might be unavailable or degraded. |
    #[serde(deserialize_with = "null_to_default")]
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct Link {
    #[serde(rename = "@odata.id")]
    odata_id: String,
    #[serde(default, rename = "href")]
    href: String,
}

impl Link {
    pub fn path(&self) -> &str {
        if self.odata_id.is_empty() {
            return self.href.as_str();
        }

        self.odata_id.as_str()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct List {
    #[serde(default)]
    pub members: Vec<Link>,
}

/// `Root` represents structure of the response body from `/redfish/v1`
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Root {
    #[serde(default)]
    pub systems: Option<Link>,
    #[serde(default)]
    pub chassis: Option<Link>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root() {
        let paths = glob::glob("tests/redfish/DSP2043_2024.3/*/index.json").unwrap();

        for path in paths.flatten() {
            let data = std::fs::read_to_string(&path).unwrap();

            match serde_json::from_slice::<Root>(data.as_bytes()) {
                Ok(_) => {}
                Err(err) => {
                    panic!("{path:?}\n{err}\n{data}");
                }
            }
        }
    }

    #[test]
    fn vendors() {
        for path in &[
            "tests/redfish/idrac9/index.json",
            "tests/redfish/xclarity/index.json",
        ] {
            let data = std::fs::read_to_string(path).unwrap();

            match serde_json::from_slice::<Root>(data.as_bytes()) {
                Ok(_) => {}
                Err(err) => {
                    panic!("{path:?}\n{err}\n{data}");
                }
            }
        }
    }
}
