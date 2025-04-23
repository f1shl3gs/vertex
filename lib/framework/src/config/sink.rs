use std::fmt::{Debug, Formatter};
use std::time::Duration;

use async_trait::async_trait;
use buffers::BufferType;
use configurable::NamedComponent;
use http::Uri;
use serde::de::{Error, MapAccess, Unexpected};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    ComponentKey, DataType, GlobalOptions, ProxyConfig, Resource, skip_serializing_if_default,
};

const fn default_timeout() -> Duration {
    Duration::from_secs(10)
}

#[derive(Clone, Debug)]
pub struct HealthcheckConfig {
    /// Whether or not to check the health of the sink when Vertex starts up.
    pub enabled: bool,

    /// The full URI to make HTTP healthcheck requests to.
    ///
    /// This must be a valid URI, which requires at least the scheme and host.
    /// All other components -- port, path, etc -- are allowed as well
    pub uri: Option<Uri>,

    /// Timeout for healthcheck
    pub timeout: Duration,
}

impl Default for HealthcheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            uri: None,
            timeout: default_timeout(),
        }
    }
}

impl Serialize for HealthcheckConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.uri {
            None => serializer.serialize_bool(self.enabled),
            Some(uri) => {
                let mut s = serializer.serialize_struct("HealthcheckConfig", 2)?;
                s.serialize_field("enabled", &self.enabled)?;
                s.serialize_field("uri", &uri.to_string())?;

                // implement skip_serialize_for_default
                if self.timeout != default_timeout() {
                    s.serialize_field("timeout", &humanize::duration::duration(&self.timeout))?;
                }

                s.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for HealthcheckConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HealthcheckConfigVisitor;

        impl<'de> serde::de::Visitor<'de> for HealthcheckConfigVisitor {
            type Value = HealthcheckConfig;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("bool or map")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(HealthcheckConfig {
                    enabled: v,
                    uri: None,
                    timeout: default_timeout(),
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                // default values
                let mut enabled = false;
                let mut uri = None;
                let mut timeout = default_timeout();

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "enabled" => {
                            enabled = map.next_value()?;
                        }
                        "uri" => {
                            let s = map.next_value::<&str>()?;
                            uri = Some(s.parse::<Uri>().map_err(|_err| {
                                Error::invalid_value(Unexpected::Str(s), &"valid uri")
                            })?);
                        }
                        "timeout" => {
                            let s = map.next_value::<&str>()?;
                            match humanize::duration::parse_duration(s) {
                                Ok(d) => timeout = d,
                                Err(_err) => {
                                    return Err(Error::invalid_value(
                                        Unexpected::Str(s),
                                        &"valid duration, like 10s",
                                    ));
                                }
                            }
                        }
                        _ => return Err(Error::unknown_field(key, &["enabled", "uri"])),
                    }
                }

                Ok(HealthcheckConfig {
                    enabled,
                    uri,
                    timeout,
                })
            }
        }

        deserializer.deserialize_any(HealthcheckConfigVisitor)
    }
}

#[derive(Clone)]
pub struct SinkContext {
    pub globals: GlobalOptions,
    pub proxy: ProxyConfig,
    pub healthcheck: HealthcheckConfig,
}

impl SinkContext {
    pub const fn proxy(&self) -> &ProxyConfig {
        &self.proxy
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_test() -> Self {
        Self {
            globals: Default::default(),
            proxy: Default::default(),
            healthcheck: Default::default(),
        }
    }
}

/// Generalized interface for describing and building sink components.
#[async_trait]
#[typetag::serde(tag = "type")]
pub trait SinkConfig: NamedComponent + Debug + Send + Sync {
    /// Builds the sink with the given context.
    async fn build(&self, cx: SinkContext) -> crate::Result<(crate::Sink, crate::Healthcheck)>;

    /// Gets the input configuration for this sink
    fn input_type(&self) -> DataType;

    /// Gets the list of resources, if any, used by this sink.
    ///
    /// Resources represent dependencies -- network ports, file descriptors, and
    /// so on -- that cannot be shared between components at runtime. This ensures
    /// that components can not be configured in a way that would deadlock the
    /// spawning of a topology, and as well, allows vertex to determine the correct
    /// order for rebuilding a topology during configuration reload when resources
    /// must first be reclaimed before being reassigned, and so on.
    fn resources(&self) -> Vec<Resource> {
        Vec::new()
    }

    /// Gets the acknowledgements configuration for this sink.
    fn acknowledgements(&self) -> bool {
        false
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SinkOuter<T> {
    pub inputs: Vec<T>,

    #[serde(flatten)]
    pub inner: Box<dyn SinkConfig>,

    #[serde(default)]
    pub buffer: buffers::BufferConfig,

    #[serde(default)]
    pub healthcheck: HealthcheckConfig,

    #[serde(default)]
    #[serde(skip_serializing_if = "skip_serializing_if_default")]
    proxy: ProxyConfig,
}

impl<T> SinkOuter<T> {
    pub fn new(inputs: Vec<T>, inner: Box<dyn SinkConfig>) -> Self {
        Self {
            inner,
            inputs,
            buffer: Default::default(),
            proxy: Default::default(),
            healthcheck: Default::default(),
        }
    }

    pub fn component_name(&self) -> &'static str {
        self.inner.component_name()
    }

    pub fn resources(&self, id: &ComponentKey) -> Vec<Resource> {
        let mut resources = self.inner.resources();

        for stage in self.buffer.stages() {
            match stage {
                BufferType::Memory { .. } => {}
                BufferType::Disk { .. } => resources.push(Resource::DiskBuffer(id.to_string())),
            }
        }

        resources
    }

    pub fn healthcheck(&self) -> HealthcheckConfig {
        self.healthcheck.clone()
    }

    pub const fn proxy(&self) -> &ProxyConfig {
        &self.proxy
    }

    pub fn with_inputs<U>(self, inputs: Vec<U>) -> SinkOuter<U> {
        SinkOuter {
            inputs,
            inner: self.inner,
            buffer: self.buffer,
            healthcheck: self.healthcheck,
            proxy: self.proxy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn healthcheck_deserialize() {
        for (input, want) in [
            (
                r#"true"#,
                HealthcheckConfig {
                    enabled: true,
                    uri: None,
                    timeout: default_timeout(),
                },
            ),
            (
                r#"false"#,
                HealthcheckConfig {
                    enabled: false,
                    uri: None,
                    timeout: default_timeout(),
                },
            ),
            (
                r#"{"enabled": true}"#,
                HealthcheckConfig {
                    enabled: true,
                    uri: None,
                    timeout: default_timeout(),
                },
            ),
            (
                r#"{"enabled": false, "uri": "http://abc"}"#,
                HealthcheckConfig {
                    enabled: false,
                    uri: Some(Uri::from_str("http://abc").unwrap()),
                    timeout: default_timeout(),
                },
            ),
            (
                r#"{"uri": "http://abc"}"#,
                HealthcheckConfig {
                    enabled: false,
                    uri: Some(Uri::from_str("http://abc").unwrap()),
                    timeout: default_timeout(),
                },
            ),
            (
                r#"{"uri": "http://abc", "timeout": "20s"}"#,
                HealthcheckConfig {
                    enabled: false,
                    uri: Some(Uri::from_str("http://abc").unwrap()),
                    timeout: Duration::from_secs(20),
                },
            ),
        ] {
            let got = serde_json::from_str::<HealthcheckConfig>(input).unwrap();
            assert_eq!(got.enabled, want.enabled);
            match (got.uri, want.uri) {
                (Some(got), Some(want)) => assert_eq!(want.to_string(), got.to_string()),
                (None, None) => {
                    // ok
                }
                (got, want) => {
                    panic!("want: {:?}, got: {:?}", want, got)
                }
            }
        }
    }
}
