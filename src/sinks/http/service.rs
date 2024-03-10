use bytes::Bytes;
use framework::http::Auth;
use framework::sink::util::http::{HttpRequest, HttpRequestBuilder};
use framework::Error;
use http::header::{CONTENT_ENCODING, CONTENT_TYPE};
use http::{HeaderName, HeaderValue, Method, Request, Uri};
use indexmap::IndexMap;

#[derive(Clone)]
pub struct HttpSinkRequestBuilder {
    method: Method,
    uri: Uri,
    auth: Option<Auth>,
    headers: IndexMap<HeaderName, HeaderValue>,
    content_type: Option<String>,
    content_encoding: Option<&'static str>,
}

impl HttpSinkRequestBuilder {
    /// Creates a new `HttpSinkRequestBuilder`
    pub fn new(
        method: Method,
        uri: Uri,
        auth: Option<Auth>,
        headers: IndexMap<HeaderName, HeaderValue>,
        content_type: Option<String>,
        content_encoding: Option<&'static str>,
    ) -> Self {
        Self {
            method,
            uri,
            auth,
            headers,
            content_type,
            content_encoding,
        }
    }
}

impl HttpRequestBuilder<()> for HttpSinkRequestBuilder {
    fn build(&self, mut req: HttpRequest<()>) -> Result<Request<Bytes>, Error> {
        let mut builder = Request::builder()
            .method(self.method.clone())
            .uri(self.uri.clone());

        if let Some(ct) = &self.content_type {
            builder = builder.header(CONTENT_TYPE, ct);
        }
        if let Some(ce) = self.content_encoding {
            builder = builder.header(CONTENT_ENCODING, ce);
        }

        let headers = builder
            .headers_mut()
            // The request building should not have errors at this point,
            // and if it did it would fail in the call to `body()` also.
            .expect("Failed to access headers in http::Request builder");
        for (key, value) in self.headers.iter() {
            headers.insert(key, value.clone());
        }

        // The request building should not have errors at this point
        let mut req = builder.body(req.take_payload())?;
        if let Some(auth) = &self.auth {
            auth.apply(&mut req);
        }

        Ok(req)
    }
}
