use bytes::Bytes;
use framework::sink::util::Compression;
use framework::sink::util::http::{HttpRequest, HttpRequestBuilder};
use http::header::{AUTHORIZATION, CONTENT_ENCODING, CONTENT_TYPE};
use http::{HeaderMap, Request};

use super::request_builder::PartitionKey;
use crate::Error;

#[derive(Clone)]
pub struct InfluxdbHttpRequestBuilder {
    uri: String,
    token: String,
    headers: HeaderMap,
    compression: Compression,
}

impl InfluxdbHttpRequestBuilder {
    #[inline]
    pub fn new(uri: String, token: String, headers: HeaderMap, compression: Compression) -> Self {
        Self {
            uri,
            token,
            headers,
            compression,
        }
    }
}

impl HttpRequestBuilder<PartitionKey> for InfluxdbHttpRequestBuilder {
    fn build(&self, mut req: HttpRequest<PartitionKey>) -> Result<Request<Bytes>, Error> {
        let key = req.metadata();
        let uri = format!("{}&bucket={}", self.uri, key.bucket);

        let mut builder = Request::post(&uri)
            .header(CONTENT_TYPE, "text/plain")
            .header(AUTHORIZATION, format!("Token {}", self.token));
        if let Some(ce) = self.compression.content_encoding() {
            builder = builder.header(CONTENT_ENCODING, ce);
        }

        for (header, value) in self.headers.iter() {
            builder = builder.header(header, value);
        }

        builder.body(req.take_payload()).map_err(Into::into)
    }
}
