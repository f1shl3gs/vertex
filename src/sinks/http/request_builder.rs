use std::io;

use bytes::Bytes;
use event::{Event, EventFinalizers, Finalizable};
use framework::sink::util::http::HttpRequest;
use framework::sink::util::{Compression, EncodeResult, RequestBuilder};

use super::encoder::HttpEncoder;

pub struct HttpRequestBuilder {
    compression: Compression,
    encoder: HttpEncoder,
}

impl HttpRequestBuilder {
    #[inline]
    pub fn new(compression: Compression, encoder: HttpEncoder) -> Self {
        Self {
            compression,
            encoder,
        }
    }
}

impl RequestBuilder<Vec<Event>> for HttpRequestBuilder {
    type Metadata = (EventFinalizers, usize);
    type Events = Vec<Event>;
    type Encoder = HttpEncoder;
    type Payload = Bytes;
    type Request = HttpRequest<()>;
    type Error = io::Error;

    fn compression(&self) -> Compression {
        self.compression
    }

    fn encoder(&self) -> &Self::Encoder {
        &self.encoder
    }

    fn split_input(&self, mut input: Vec<Event>) -> (Self::Metadata, Self::Events) {
        let finalizers = input.take_finalizers();
        ((finalizers, input.len()), input)
    }

    fn build_request(
        &self,
        metadata: Self::Metadata,
        payload: EncodeResult<Self::Payload>,
    ) -> Self::Request {
        HttpRequest::new(payload.payload, metadata.0, ())
    }
}
