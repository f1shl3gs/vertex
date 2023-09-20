use std::fmt::{Display, Formatter};

use bytes::BytesMut;
use event::Event;
use tokio_util::codec::Encoder as _;
use tracing::warn;

use super::{
    CharacterDelimitedEncoder, Framer, FramingError, NewlineDelimitedEncoder, SerializeError,
    Serializer, TextSerializer,
};

/// An error that occurred while encoding structured events into byte frames.
#[derive(Debug)]
pub enum EncodingError {
    /// The error occurred while encoding the byte frame boundaries.
    Framing(FramingError),
    /// The error occurred while serializing the byte frame.
    Serialize(SerializeError),

    /// The error occurred while framing or serializing.
    Io(std::io::Error),
}

impl From<std::io::Error> for EncodingError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl Display for EncodingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodingError::Framing(err) => write!(f, "Framing failed, {:?}", err),
            EncodingError::Serialize(err) => write!(f, "Serializing failed, {:?}", err),
            EncodingError::Io(err) => write!(f, "IO error, {}", err),
        }
    }
}

impl std::error::Error for EncodingError {}

/// An encoder that can encode structured events into byte frames.
#[derive(Clone, Debug)]
pub struct Encoder<F>
where
    F: Clone,
{
    framer: F,
    serializer: Serializer,
}

impl Default for Encoder<Framer> {
    fn default() -> Self {
        Self {
            framer: Framer::NewlineDelimited(NewlineDelimitedEncoder::new()),
            serializer: Serializer::Text(TextSerializer::new()),
        }
    }
}

impl Default for Encoder<()> {
    fn default() -> Self {
        Self {
            framer: (),
            serializer: Serializer::Text(TextSerializer::new()),
        }
    }
}

impl<F> Encoder<F>
where
    F: Clone,
{
    /// Serialize the event without applying framing.
    pub fn serialize(&mut self, event: Event, buf: &mut BytesMut) -> Result<(), EncodingError> {
        let len = buf.len();
        let mut payload = buf.split_off(len);

        self.serialize_at_start(event, &mut payload)?;

        buf.unsplit(payload);

        Ok(())
    }

    fn serialize_at_start(
        &mut self,
        event: Event,
        buf: &mut BytesMut,
    ) -> Result<(), EncodingError> {
        self.serializer.encode(event, buf).map_err(|err| {
            warn!(
                message = "Failed serializing frame",
                ?err,
                internal_log_rate_limit = true
            );

            EncodingError::Serialize(err)
        })
    }
}

impl Encoder<Framer> {
    /// Creates a new `Encoder` with the specified `Serializer` to produce bytes from
    /// a structured event, and the `Framer` to wrap these into a byte frame.
    pub const fn new(framer: Framer, serializer: Serializer) -> Self {
        Self { framer, serializer }
    }

    /// Get the prefix that encloses a batch of events.
    pub const fn batch_prefix(&self) -> &[u8] {
        match (&self.framer, &self.serializer) {
            (
                Framer::CharacterDelimited(CharacterDelimitedEncoder { delimiter: b',' }),
                Serializer::Json(_) | Serializer::Native(_),
            ) => b"[",
            _ => &[],
        }
    }

    /// Get the suffix that encloses a batch of events.
    pub const fn batch_suffix(&self) -> &[u8] {
        match (&self.framer, &self.serializer) {
            (
                Framer::CharacterDelimited(CharacterDelimitedEncoder { delimiter: b',' }),
                Serializer::Json(_) | Serializer::Native(_),
            ) => b"]",
            _ => &[],
        }
    }

    /// Get the HTTP content type
    pub const fn content_type(&self) -> &'static str {
        match (&self.serializer, &self.framer) {
            (Serializer::Json(_) | Serializer::Native(_), Framer::NewlineDelimited(_)) => {
                "application/x-ndjson"
            }
            (
                Serializer::Json(_) | Serializer::Native(_),
                Framer::CharacterDelimited(CharacterDelimitedEncoder { delimiter: b',' }),
            ) => "application/json",
            (Serializer::Native(_), _) => "application/octet-stream",
            (Serializer::Json(_) | Serializer::Logfmt(_) | Serializer::Text(_), _) => "text/plain",
        }
    }
}

impl Encoder<()> {
    /// Creates a new `Encoder` with the specified `Serializer` to produce
    /// bytes from a structured event.
    pub const fn new(serializer: Serializer) -> Self {
        Self {
            framer: (),
            serializer,
        }
    }
}

impl tokio_util::codec::Encoder<Event> for Encoder<Framer> {
    type Error = EncodingError;

    fn encode(&mut self, event: Event, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let len = buf.len();
        let mut payload = buf.split_off(len);

        self.serialize_at_start(event, &mut payload)?;

        // Frame the serialized event.
        self.framer.encode((), &mut payload).map_err(|err| {
            warn!(
                message = "Failed framing bytes",
                ?err,
                internal_log_rate_limit = true
            );
            EncodingError::Framing(err)
        })?;

        buf.unsplit(payload);

        Ok(())
    }
}

impl tokio_util::codec::Encoder<Event> for Encoder<()> {
    type Error = EncodingError;

    fn encode(&mut self, event: Event, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let len = buf.len();
        let mut payload = buf.split_off(len);

        self.serialize_at_start(event, &mut payload)?;

        buf.unsplit(payload);

        Ok(())
    }
}
