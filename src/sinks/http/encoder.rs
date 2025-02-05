use std::io::Write;

use bytes::BytesMut;
use codecs::encoding::{Framer, Transformer};
use event::Event;
use framework::sink::util::Encoder;
use tokio_util::codec::Encoder as _;

pub struct HttpEncoder {
    encoder: codecs::Encoder<Framer>,
    transformer: Transformer,
}

impl HttpEncoder {
    #[inline]
    pub const fn new(encoder: codecs::Encoder<Framer>, transformer: Transformer) -> Self {
        Self {
            encoder,
            transformer,
        }
    }
}

impl Encoder<Vec<Event>> for HttpEncoder {
    fn encode(&self, events: Vec<Event>, writer: &mut dyn Write) -> std::io::Result<usize> {
        let mut encoder = self.encoder.clone();

        let data_size: usize = events.iter().map(|event| event.size_of()).sum();
        let mut buf = BytesMut::with_capacity(data_size * 2);

        for mut event in events {
            self.transformer.transform(&mut event);

            encoder.encode(event, &mut buf).map_err(|_err| {
                std::io::Error::new(std::io::ErrorKind::Other, "unable to encode event")
            })?;
        }

        let data = buf.freeze();
        writer.write_all(&data)?;

        Ok(data.len())
    }
}
