use async_trait::async_trait;
use crate::{signal, config::{
    deserialize_duration,
    serialize_duration,
}, SignalHandler};
use url::Url;
use serde::{Deserialize, Serialize};
use indexmap::map::IndexMap;
use crate::tls::TLSOptions;
use super::builder::Builder;

pub type Result<T> = std::result::Result<T, Vec<String>>;

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait ProviderConfig: core::fmt::Debug + Send + Sync + dyn_clone::DynClone {
    /// Builds a provider, returning a string containing the config. It's passed
    /// a signals cahannel to control reloading and shutdown, as applicable.
    async fn build(&mut self, signal_handler: &mut signal::SignalHandler) -> Result<Builder>;

    fn provider_type(&self) -> &'static str;
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RequestConfig {
    #[serde(default)]
    pub headers: IndexMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct HTTPConfig {
    url: Option<Url>,
    request: RequestConfig,
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    interval: chrono::Duration,
    tls: Option<TLSOptions>,
}

impl Default for HTTPConfig {
    fn default() -> Self {
        Self {
            url: None,
            request: RequestConfig::default(),
            interval: chrono::Duration::seconds(30),
            tls: None,
        }
    }
}

#[async_trait]
#[typetag::serde(name = "http")]
impl ProviderConfig for HTTPConfig {
    async fn build(&mut self, signal_handler: &mut SignalHandler) -> Result<Builder> {
        todo!()
    }

    fn provider_type(&self) -> &'static str {
        todo!()
    }
}

async fn http_request(
    url: &Url,
    tls: &Option<TLSOptions>,
    headers: &IndexMap<String, String>,
) -> std::result::Result<bytes::Bytes, &'static str> {
    todo!()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileConfig {
    path: String,
}

#[async_trait]
#[typetag::serde(name = "file")]
impl ProviderConfig for FileConfig {
    async fn build(&mut self, signal_handler: &mut SignalHandler) -> Result<Builder> {
        todo!()
    }

    fn provider_type(&self) -> &'static str {
        todo!()
    }
}