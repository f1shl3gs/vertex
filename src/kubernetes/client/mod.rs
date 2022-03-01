mod config;

use http::{HeaderValue, uri};
use framework::config::ProxyConfig;
use framework::http::HttpClient;

/// A client to the k8s API
///
/// Wraps our in-house `HttpClient`
#[derive(Clone, Debug)]
pub struct Client {
    inner: HttpClient,
    scheme: uri::Scheme,
    authority: uri::Authority,
    header: Option<HeaderValue>,
}

impl Client {
    /// Create a new `Client`
    ///
    /// Takes the common kubernetes API cluster configuration `Config`.
    ///
    /// Consumes the configuration to populate the internal state.
    /// Returns an error if the configuration is not valid.
    pub fn new(config: config::Config, proxy: &ProxyConfig) -> framework::Result<Self> {

    }
}