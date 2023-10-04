use bytes::Bytes;
use event::{EventFinalizers, Finalizable, Metric};
use framework::partition::Partitioner;
use framework::sink::util::{Compression, EncodeResult, RequestBuilder};
use framework::template::Template;

use super::encoder::LineProtocolEncoder;
use super::service::InfluxdbRequest;

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct PartitionKey {
    bucket: String,
    org: String,
}

/// KeyPartitioner that partitions events by (org, bucket) pair.
pub struct KeyPartitioner {
    org: Template,
    bucket: Template,
}

impl KeyPartitioner {
    pub fn new(org: Template, bucket: Template) -> Self {
        Self { org, bucket }
    }
}

impl Partitioner for KeyPartitioner {
    type Item = Metric;
    type Key = Option<PartitionKey>;

    fn partition(&self, item: &Self::Item) -> Self::Key {
        let org = self.org.render(item).ok()?;
        let bucket = self.bucket.render(item).ok()?;

        Some(PartitionKey {
            bucket: String::from_utf8_lossy(&org).to_string(),
            org: String::from_utf8_lossy(&bucket).to_string(),
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
    type Metadata = (PartitionKey, EventFinalizers);
    type Events = Vec<Metric>;
    type Encoder = LineProtocolEncoder;
    type Payload = Bytes;
    type Request = InfluxdbRequest;
    type Error = std::io::Error;

    fn compression(&self) -> Compression {
        self.compression
    }

    fn encoder(&self) -> &Self::Encoder {
        &self.encoder
    }

    fn split_input(&self, input: (PartitionKey, Vec<Metric>)) -> (Self::Metadata, Self::Events) {
        let (pk, mut metrics) = input;
        let finalizers = metrics.take_finalizers();
        ((pk, finalizers), metrics)
    }

    fn build_request(
        &self,
        metadata: Self::Metadata,
        payload: EncodeResult<Self::Payload>,
    ) -> Self::Request {
        let (pk, finalizers) = metadata;
        // TODO: fix this
        let batch_size = finalizers.len();

        InfluxdbRequest {
            org: pk.org,
            bucket: pk.bucket,
            compression: self.compression,
            finalizers,
            data: payload.payload,
            batch_size,
        }
    }
}
