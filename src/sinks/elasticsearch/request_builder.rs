use bytes::Bytes;
use event::EventFinalizers;
use framework::sink::util::{Compression, RequestBuilder};

use crate::sinks::elasticsearch::encoder::{ElasticsearchEncoder, ProcessedEvent};

#[derive(Debug)]
pub struct ElasticsearchRequestBuilder {
    pub compression: Compression,
    pub encoder: ElasticsearchEncoder,
}

pub struct Metadata {
    finalizers: EventFinalizers,
    batch_size: usize,
    events_byte_size: usize,
}

impl RequestBuilder<Vec<ProcessedEvent>> for ElasticsearchRequestBuilder {
    type Metadata = Metadata;
    type Events = Vec<ProcessedEvent>;
    type Encoder = ElasticsearchEncoder;
    type Payload = Bytes;
    type Request = ElasticsearchRequest;
    type Error = std::io::Error;

    fn compression(&self) -> Compression {
        todo!()
    }

    fn encoder(&self) -> &Self::Encoder {
        todo!()
    }

    fn split_input(&self, input: Vec<ProcessedEvent>) -> (Self::Metadata, Self::Events) {
        todo!()
    }

    fn build_request(&self, metadata: Self::Metadata, payload: Self::Payload) -> Self::Request {
        todo!()
    }
}
