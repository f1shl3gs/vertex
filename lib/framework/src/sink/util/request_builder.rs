use std::io;

use bytes::Bytes;

use super::{Compression, Compressor, Encoder};

/// Metadata for batch requests.
pub struct RequestMetadata {
    /// Number of events represented by this batch request.
    pub event_count: usize,
    /// Size, in bytes, of the in-memory representation of all events in this batch request.
    pub events_bytes: usize,
    /// Uncompressed size, in bytes, of the encoded events in this batch request.
    pub request_encoded_size: usize,
    /// On-the-wire size, in bytes, of the batch request itself after compression, etc.
    ///
    /// This is akin to the bytes sent/received over the network, regardless of whether or not
    /// compression was used.
    pub request_wire_size: usize,
}

impl RequestMetadata {
    pub fn new(
        event_count: usize,
        events_bytes: usize,
        request_encoded_size: usize,
        request_wire_size: usize,
    ) -> Self {
        Self {
            event_count,
            events_bytes,
            request_encoded_size,
            request_wire_size,
        }
    }
}

pub struct EncodeResult<P> {
    pub data: P,
    pub uncompressed_byte_size: usize,
    pub compressed_byte_size: Option<usize>,
}

impl<P> EncodeResult<P>
where
    P: AsRef<[u8]>,
{
    pub fn uncompressed(data: P) -> Self {
        let uncompressed_byte_size = data.as_ref().len();

        Self {
            data,
            uncompressed_byte_size,
            compressed_byte_size: None,
        }
    }

    pub fn compressed(data: P, uncompressed_byte_size: usize) -> Self {
        let compressed_byte_size = data.as_ref().len();

        Self {
            data,
            uncompressed_byte_size,
            compressed_byte_size: Some(compressed_byte_size),
        }
    }
}

impl<P> EncodeResult<P> {
    // Can't be `const` because you can't (yet?) run deconstructors in a const context, which is what this function does
    // by dropping the (un)compressed sizes.
    pub fn into_payload(self) -> P {
        self.data
    }
}

/// Generalized interface for defining how a batch of events will be turned into a request.
pub trait RequestBuilder<Input> {
    type Metadata;
    type Events;
    type Encoder: Encoder<Self::Events>;
    type Payload: From<Bytes> + AsRef<[u8]>;
    type Request;
    type Error: From<io::Error>;

    /// Gets the compression algorithm used by this request builder
    fn compression(&self) -> Compression;

    /// Gets the encoder used by this request builder
    fn encoder(&self) -> &Self::Encoder;

    /// Splits apart the input into the metadata and event portions.
    ///
    /// The metadata should be any information that needs to be passed back to `build_request`
    /// as-is, such as event finalizers, while the events are the actual events to process.
    fn split_input(&self, input: Input) -> (Self::Metadata, Self::Events);

    fn encode_events(
        &self,
        events: Self::Events,
    ) -> Result<EncodeResult<Self::Payload>, Self::Error> {
        // TODO: Should we add enough bounds on `Self::Events` that we could automatically derive
        //   event count/event byte size, and then we could generate `BatchRequestMetadata` and
        //   pass it directly to `build_request`? That would obviate needing to wrap `payload`
        //   in `EncodeResult`, although practically speaking.. the name would be kind of
        //   clash-y with `Self::Metadata`
        let mut compressor = Compressor::from(self.compression());
        let compressed = compressor.is_compressed();
        let _ = self.encoder().encode(events, &mut compressor)?;

        let payload = compressor.into_inner().freeze();
        let result = if compressed {
            let compressed_byte_size = payload.len();
            EncodeResult::compressed(payload.into(), compressed_byte_size)
        } else {
            EncodeResult::uncompressed(payload.into())
        };

        Ok(result)
    }

    /// Builds a request for the given metadata and payload
    fn build_request(
        &self,
        metadata: Self::Metadata,
        payload: EncodeResult<Self::Payload>,
    ) -> Self::Request;
}

/// Generalized interface for defining how a batch of events will incrementally be turned into
/// requests.
///
/// As opposed to `RequestBuilder`, this trait provides the means to incrementally build requests
/// from a single batch of events, where all events in the batch may not fit into a single
/// request. This can be important for sinks where the underlying service has limitations on the
/// size of a request, or how many events may be present, necessitating a batch be split up
/// into multiple requests.
///
/// While batches can be limited in size before being handed off to a request builder, we can't
/// always know in advance how large the encoded payload will be, which requires us to be able
/// to potentially split a batch into multiple requests
pub trait IncrementalRequestBuilder<Input> {
    type Metadata;
    type Payload;
    type Request;
    type Error;

    /// Incrementally encodes the given input, potentially generating multiple payloads.
    fn encode_events_incremental(
        &mut self,
        input: Input,
    ) -> Vec<Result<(Self::Metadata, Self::Payload), Self::Error>>;

    /// Builds a request for the given metadata and payload
    fn build_request(&mut self, metadata: Self::Metadata, payload: Self::Payload) -> Self::Request;
}
