use framework::tls::TlsOptions;
use http::Uri;

/// A k8s client configuration
///
/// This type is designed to hold all possible variants of the configuration.
/// It also abstracts the client from the various ways to obtain the
/// configuration.
///
/// The implementation is fairly limited, and only covers the use cases we
/// support.
#[derive(Clone, Debug)]
pub struct Config {
    /// The base URL to use when constructing HTTP requests to the k8s API
    pub base: Uri,

    /// The bearer token to use at the `Authorization` header
    pub token: Option<String>,

    /// The TLS configuration parameters to use at the HTTP client
    pub tls_options: TlsOptions,
}
