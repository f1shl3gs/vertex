mod config;

use framework::config::ProxyConfig;
use framework::http::{HttpClient, HttpError};
use framework::tls::TlsSettings;
use http::{header, uri, HeaderValue, Request, Response, Uri};
use hyper::Body;

/// A client to the k8s API
///
/// Wraps our in-house `HttpClient`
#[derive(Clone, Debug)]
pub struct Client {
    inner: HttpClient,
    scheme: uri::Scheme,
    authority: uri::Authority,
    auth_header: Option<HeaderValue>,
}

impl Client {
    /// Create a new `Client`
    ///
    /// Takes the common kubernetes API cluster configuration `Config`.
    ///
    /// Consumes the configuration to populate the internal state.
    /// Returns an error if the configuration is not valid.
    pub fn new(config: config::Config, proxy: &ProxyConfig) -> framework::Result<Self> {
        let config::Config {
            base,
            tls_options,
            token,
        } = config;

        let tls_settings = TlsSettings::from_options(&Some(tls_options))?;
        let inner = HttpClient::new(tls_settings, proxy)?;

        let uri::Parts {
            scheme, authority, ..
        } = base.into_parts();

        let scheme = scheme.ok_or("no scheme")?;
        let authority = authority.ok_or("no authority")?;

        let auth_header = match &token {
            Some(t) => Some(HeaderValue::from_str(format!("Bearer {}", t).as_str())?),
            None => None,
        };

        Ok(Self {
            inner,
            scheme,
            authority,
            auth_header,
        })
    }

    /// Alters a request according to the client configuration and sends it.
    pub async fn send<B: Into<Body>>(
        &mut self,
        req: Request<B>,
    ) -> Result<Response<Body>, HttpError> {
        let req = self.prepare_request(req);
        self.inner.send(req).await
    }

    fn prepare_request<B: Into<Body>>(&self, req: Request<B>) -> Request<Body> {
        let (mut parts, body) = req.into_parts();
        let body = body.into();

        parts.uri = self.adjust_uri(parts.uri);
        if let Some(ah) = self.auth_header.as_ref() {
            parts.headers.insert(header::AUTHORIZATION, ah.clone());
        }

        Request::from_parts(parts, body)
    }

    fn adjust_uri(&self, uri: Uri) -> Uri {
        let mut parts = uri.into_parts();
        parts.scheme = Some(self.scheme.clone());
        parts.authority = Some(self.authority.clone());
        Uri::from_parts(parts).unwrap()
    }
}
