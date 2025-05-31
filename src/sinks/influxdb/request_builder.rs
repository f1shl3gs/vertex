use bytes::Bytes;
use event::{EventFinalizers, Finalizable, Metric};
use framework::partition::Partitioner;
use framework::sink::http::HttpRequest;
use framework::sink::util::{Compression, EncodeResult, RequestBuilder};
use framework::template::Template;

use super::encoder::LineProtocolEncoder;

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct PartitionKey {
    pub bucket: String,
}

/// KeyPartitioner that partitions events by (org, bucket) pair.
pub struct KeyPartitioner {
    bucket: Template,
}

impl KeyPartitioner {
    pub fn new(bucket: Template) -> Self {
        Self { bucket }
    }
}

impl Partitioner for KeyPartitioner {
    type Item = Metric;
    type Key = Option<PartitionKey>;

    fn partition(&self, item: &Self::Item) -> Self::Key {
        let bucket = self.bucket.render(item).ok()?;

        Some(PartitionKey {
            bucket: String::from_utf8_lossy(&bucket).to_string(),
        })
    }
}

pub struct InfluxdbRequestBuilder {
    compression: Compression,
    encoder: LineProtocolEncoder,
}

impl InfluxdbRequestBuilder {
    pub fn new(compression: Compression) -> Self {
        Self {
            compression,
            encoder: LineProtocolEncoder,
        }
    }
}

impl RequestBuilder<(PartitionKey, Vec<Metric>)> for InfluxdbRequestBuilder {
    type Metadata = (PartitionKey, EventFinalizers, usize);
    type Events = Vec<Metric>;
    type Encoder = LineProtocolEncoder;
    type Payload = Bytes;
    type Request = HttpRequest<PartitionKey>;
    type Error = std::io::Error;

    fn compression(&self) -> Compression {
        self.compression
    }

    fn encoder(&self) -> &Self::Encoder {
        &self.encoder
    }

    fn split_input(&self, input: (PartitionKey, Vec<Metric>)) -> (Self::Metadata, Self::Events) {
        let (partition_key, mut metrics) = input;
        let finalizers = metrics.take_finalizers();
        ((partition_key, finalizers, metrics.len()), metrics)
    }

    fn build_request(
        &self,
        metadata: Self::Metadata,
        payload: EncodeResult<Self::Payload>,
    ) -> Self::Request {
        let (pk, finalizers, batch_size) = metadata;
        let events_byte_size = payload.data.len();

        HttpRequest::new(payload.data, finalizers, batch_size, events_byte_size, pk)
    }
}
