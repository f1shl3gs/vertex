use super::StreamDecodingError;
use crate::codecs::framing::newline::NewlineDelimitedDecoder;
use crate::codecs::BytesDeserializer;
use bytes::{Bytes, BytesMut};
use dyn_clone::DynClone;
use event::Event;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::fmt::{Debug, Formatter};
use tokio_util::codec::LinesCodecError;

/// An error that occurred while decoding structured events from a byte stream
/// byte message
#[derive(Debug)]
pub enum Error {
    /// The error occurred while producing byte frames from the byte
    /// stream byte messages
    Framing(BoxedFramingError),
    /// The error occurred while parsing structured events from a byte frame
    Parsing(crate::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Framing(err) => write!(f, "Framing({})", err),
            Self::Parsing(err) => write!(f, "Parsing({})", err),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Framing(Box::new(err))
    }
}

impl StreamDecodingError for Error {
    fn can_continue(&self) -> bool {
        match self {
            Self::Framing(err) => err.can_continue(),
            Self::Parsing(_) => true,
        }
    }
}

/// An error that occurred while producing byte frames from a byte stream byte
/// message.
///
/// It requires conformance to `TcpError` so that we can determine whether the
/// error is recoverable or if trying to continue will lead to hanging up the
/// TCP source indefinitely.
pub trait FramingError: std::error::Error + StreamDecodingError + Send + Sync {}

impl std::error::Error for BoxedFramingError {}

impl StreamDecodingError for std::io::Error {
    fn can_continue(&self) -> bool {
        false
    }
}

impl FramingError for std::io::Error {}

impl FramingError for LinesCodecError {}

impl StreamDecodingError for LinesCodecError {
    fn can_continue(&self) -> bool {
        match self {
            LinesCodecError::MaxLineLengthExceeded => true,
            LinesCodecError::Io(err) => err.can_continue(),
        }
    }
}

impl From<std::io::Error> for BoxedFramingError {
    fn from(err: std::io::Error) -> Self {
        Box::new(err)
    }
}

impl From<LinesCodecError> for BoxedFramingError {
    fn from(err: LinesCodecError) -> Self {
        Box::new(err)
    }
}

/// A `Box` containing a `FramingError`
pub type BoxedFramingError = Box<dyn FramingError>;

impl StreamDecodingError for BoxedFramingError {
    fn can_continue(&self) -> bool {
        self.as_ref().can_continue()
    }
}

/// Produce byte frames from a byte stream / byte message
pub trait Framer:
    tokio_util::codec::Decoder<Item = Bytes, Error = BoxedFramingError> + DynClone + Debug + Send + Sync
{
}

/// Default implementation for `Framer`s that implement
/// `tokio_util::codec::Decoder`
impl<Decoder> Framer for Decoder where
    Decoder: tokio_util::codec::Decoder<Item = Bytes, Error = BoxedFramingError>
        + Clone
        + Debug
        + Send
        + Sync
{
}

dyn_clone::clone_trait_object!(Framer);

/// A `Box` containing a `Frame`
pub type BoxedFramer = Box<dyn Framer>;

/// Define options for a framer and build it from the config object.
///
/// Implementors must annotate the struct with `#[typetag::serde(name = "...")]`
/// to define which value should be read from the `method` key to select their
/// implementation
#[typetag::serde(tag = "method")]
pub trait FramingConfig: Debug + DynClone + Send + Sync {
    /// Build a framer from this configuration
    ///
    /// Fails if the configuration is invalid
    fn build(&self) -> crate::Result<BoxedFramer>;
}

dyn_clone::clone_trait_object!(FramingConfig);

/// Parse structured eevnts from bytes
pub trait Deserializer: DynClone + Debug + Send + Sync {
    /// Parses structured eevnts from bytes
    ///
    /// It returns a `SmallVec` rather than an `Event` directly, since one byte
    /// frame can potentially hold multiple events, e.g. when parsing a JSON
    /// array. However, we optimize the most common case of emitting one event
    /// by not requiring heap allocations for it
    fn parse(&self, bytes: Bytes) -> crate::Result<SmallVec<[Event; 1]>>;
}

dyn_clone::clone_trait_object!(Deserializer);

/// A `Box` containing a `Deserializer`
pub type BoxedDeserializer = Box<dyn Deserializer>;

/// Define options for a deserializer and build it from the config object
///
/// Implementors must annotate the struct with `#[typetag::serde(name = "...")]`
/// to define which value should be read from the `codec` key to select their
/// implementation.
#[typetag::serde(tag = "codec")]
pub trait DeserializerConfig: Debug + DynClone + Send + Sync {
    /// Builds a deserializer from this configuration
    ///
    /// Fails if the configuration is invalid
    fn build(&self) -> crate::Result<BoxedDeserializer>;
}

dyn_clone::clone_trait_object!(DeserializerConfig);

/// A decoder that can decode structured events from a byte stream / byte
/// message
#[derive(Debug, Clone)]
pub struct Decoder {
    framer: BoxedFramer,
    deserializer: BoxedDeserializer,
}

impl Default for Decoder {
    fn default() -> Self {
        Self {
            framer: Box::new(NewlineDelimitedDecoder::new()),
            deserializer: Box::new(BytesDeserializer::new()),
        }
    }
}

impl Decoder {
    /// Creates a new `Decoder` with the specified `Framer` to produce byte
    /// frames from the byte stream / byte messages and `Deserializer` to parse
    /// structured events from a byte frame
    pub fn new(framer: BoxedFramer, deserializer: BoxedDeserializer) -> Self {
        Self {
            framer,
            deserializer,
        }
    }

    /// Handles the framing result and parses it into a structured event, if
    /// possible
    ///
    /// Emits logs if either framing or parsing failed
    fn handle_framing_result(
        &mut self,
        frame: Result<Option<Bytes>, BoxedFramingError>,
    ) -> Result<Option<(SmallVec<[Event; 1]>, usize)>, Error> {
        let frame = frame.map_err(|err| {
            warn!(
                message = "Failed framing bytes",
                %err,
                internal_log_rate_secs = 10
            );
            Error::Framing(err)
        })?;

        let frame = match frame {
            Some(frame) => frame,
            _ => return Ok(None),
        };

        let byte_size = frame.len();

        // Parse structured events from the byte frame
        self.deserializer
            .parse(frame)
            .map(|event| Some((event, byte_size)))
            .map_err(|err| {
                warn!(
                    message = "Failed deserializing frame",
                    %err,
                    internal_log_rate_secs = 10
                );

                Error::Parsing(err)
            })
    }
}

impl tokio_util::codec::Decoder for Decoder {
    type Item = (SmallVec<[Event; 1]>, usize);
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let frame = self.framer.decode(src);
        self.handle_framing_result(frame)
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let frame = self.framer.decode_eof(buf);
        self.handle_framing_result(frame)
    }
}

/// Config used to build a `Decoder`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DecodingConfig {
    /// The framing config
    framing: Box<dyn FramingConfig>,
    /// The decoding config
    decoding: Box<dyn DeserializerConfig>,
}

impl DecodingConfig {
    /// Creates a new `DecodingConfig` with the provided `FramingConfig` and `DeserializerConfig`
    pub fn new(framing: Box<dyn FramingConfig>, decoding: Box<dyn DeserializerConfig>) -> Self {
        Self { framing, decoding }
    }

    /// Builds a `Decoder` from the provided configuration
    pub fn build(&self) -> crate::Result<Decoder> {
        // Build the framer
        let framer: BoxedFramer = self.framing.build()?;

        // Build the deserializer
        let deserializer: BoxedDeserializer = self.decoding.build()?;

        Ok(Decoder::new(framer, deserializer))
    }
}
