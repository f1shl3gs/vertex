mod msgpack;

use std::fmt::Formatter;
use std::io::Read;
use std::net::SocketAddr;
use std::time::Duration;

use bytes::{Buf, Bytes, BytesMut};
use chrono::DateTime;
use codecs::decoding::StreamDecodingError;
use configurable::configurable_component;
use event::{Events, LogRecord};
use flate2::read::MultiGzDecoder;
use framework::Source;
use framework::config::{OutputType, Resource, SourceConfig, SourceContext};
use framework::source::tcp::{SocketListenAddr, TcpSource, TcpSourceAck, TcpSourceAcker};
use framework::tcp::TcpKeepaliveConfig;
use framework::tls::TlsConfig;
use msgpack::{Error as MsgPackError, ReadExt, decode_string};
use msgpack::{decode_binary, decode_entry, decode_options, decode_value};
use value::path;

fn default_listen() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 24224))
}

/// Collect logs from a Fluentd or Fluent Bit agent.
#[configurable_component(source, name = "fluent")]
struct Config {
    /// The socket address to listen for connections
    #[serde(default = "default_listen")]
    listen: SocketAddr,

    tls: Option<TlsConfig>,

    /// The maximum number of TCP connections that are allowed at any given time.
    connection_limit: Option<usize>,

    keepalive: Option<TcpKeepaliveConfig>,

    /// The size of the received buffer used for each connection.
    #[serde(default, with = "humanize::bytes::serde_option")]
    receive_buffer: Option<usize>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "fluent")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let source = ForwardSource;

        source.run(
            SocketListenAddr::SocketAddr(self.listen),
            self.keepalive,
            Duration::from_secs(30),
            self.tls.as_ref(),
            self.receive_buffer,
            cx,
            None,
        )
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::log()]
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.listen)]
    }

    fn can_acknowledge(&self) -> bool {
        true
    }
}

fn encode_ack_resp(chunk: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5 + 1 + chunk.len());
    #[rustfmt::skip]
    buf.extend([
        0x81, // fixmap and length is 1
        0xa3, 0x61, 0x63, 0x6b, // ack
    ]);

    let len = chunk.len();
    if len < 32 {
        // positive fixint
        let mark = len as u8;
        buf.push(mark);
    } else if len <= u8::MAX as usize {
        // str 8
        buf.push(0xd9);
        buf.push(len as u8);
    } else if len <= u16::MAX as usize {
        // str 16
        buf.push(0xda);
        let bl = u16::to_be_bytes(len as u16);
        buf.extend_from_slice(&bl);
    } else {
        // str 32
        buf.push(0xdb);
        let bl = u32::to_be_bytes(len as u32);
        buf.extend_from_slice(&bl);
    };

    buf.extend_from_slice(chunk.as_ref());

    buf
}

#[derive(Debug)]
enum DecodeError {
    IO(std::io::Error),
    Decode(msgpack::Error),
    Decompression,
}

impl From<std::io::Error> for DecodeError {
    fn from(err: std::io::Error) -> Self {
        DecodeError::IO(err)
    }
}

impl From<msgpack::Error> for DecodeError {
    fn from(err: msgpack::Error) -> Self {
        match err {
            msgpack::Error::IO(err) => DecodeError::IO(err),
            err => DecodeError::Decode(err),
        }
    }
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::IO(err) => write!(f, "io error: {err}"),
            DecodeError::Decode(err) => err.fmt(f),
            DecodeError::Decompression => f.write_str("decompression entries failed"),
        }
    }
}

impl StreamDecodingError for DecodeError {
    fn can_continue(&self) -> bool {
        match self {
            DecodeError::IO(_) => false,
            DecodeError::Decode(_) => false,
            DecodeError::Decompression => false,
        }
    }
}

struct ForwardDecoder;

impl ForwardDecoder {
    fn decode_internal<R: Read>(
        &self,
        reader: &mut R,
    ) -> Result<Option<ForwardFrame>, DecodeError> {
        // all the received events is an array
        //
        // https://github.com/fluent/fluentd/wiki/Forward-Protocol-Specification-v1#event-modes
        let len = match reader.read_u8()? {
            // fixarray
            typ @ 0x90..=0x9f => (typ & 0x0f) as usize,
            // array 16
            0xdc => reader.read_u16()? as usize,
            // array 32
            0xdd => reader.read_u32()? as usize,

            // heartbeat
            0xc0 => return Ok(None),

            typ => return Err(MsgPackError::UnknownType(typ, "message").into()),
        };

        if !(2..=4).contains(&len) {
            return Err(MsgPackError::IO(std::io::ErrorKind::InvalidData.into()).into());
        }

        // the first part
        let tag = decode_string(reader)?;

        let typ = reader.read_u8()?;
        match typ {
            // Forward Mode's second part is array
            //
            // https://github.com/fluent/fluentd/wiki/Forward-Protocol-Specification-v1#forward-mode
            0x90..=0x9f | 0xdc | 0xdd => {
                // array
                let arr_len = match typ {
                    0x90..=0x9f => (typ & 0x0f) as usize,
                    0xdc => reader.read_u16()? as usize,
                    0xdd => reader.read_u32()? as usize,
                    _ => unreachable!(),
                };

                let mut logs = Vec::with_capacity(arr_len);
                for _ in 0..arr_len {
                    let (timestamp, value) = decode_entry(reader)?;
                    let mut log = LogRecord::from(value);

                    let metadata = log.metadata_mut().value_mut();

                    metadata.insert(path!("fluent", "timestamp"), timestamp);
                    metadata.insert(path!("fluent", "tag"), tag.clone());

                    logs.push(log);
                }

                if len == 3 {
                    // options
                    let options = decode_options(reader)?;

                    Ok(Some(ForwardFrame {
                        logs,
                        chunk: options.chunk,
                    }))
                } else {
                    Ok(Some(ForwardFrame { logs, chunk: None }))
                }
            }

            // PackedForward
            0xa0..=0xbf | 0xd9..=0xdb | 0xc4..=0xc6 => {
                let data = match typ {
                    // PackedForward's second part could be str
                    //
                    // https://github.com/fluent/fluentd/wiki/Forward-Protocol-Specification-v1#packedforward-mode
                    0xa0..=0xbf | 0xd9..=0xdb => {
                        // Client may send a `MessagePackEventStream` as msgpack `str` format
                        // for compatibility reasons.
                        let str_len = match typ {
                            // fix str
                            0xa0..=0xbf => (typ & 0x1f) as usize,
                            // str 8
                            0xd9 => reader.read_u8()? as usize,
                            // str 16
                            0xda => reader.read_u16()? as usize,
                            // str 32
                            0xdb => reader.read_u32()? as usize,
                            _ => unreachable!(),
                        };

                        let mut data = vec![0; str_len];
                        reader.read_exact(&mut data)?;

                        data
                    }
                    // bin
                    0xc4..=0xd6 => decode_binary(reader)?,
                    _ => unreachable!(),
                };

                let option = if len == 3 {
                    Some(decode_options(reader)?)
                } else {
                    None
                };
                let compressed = option.as_ref().map(|o| o.compressed).unwrap_or_default();

                let buf = if compressed {
                    let mut buf = Vec::new();
                    MultiGzDecoder::new(data.as_slice())
                        .read_to_end(&mut buf)
                        .map(|_| buf)
                        .map_err(|_err| DecodeError::Decompression)?
                } else {
                    data
                };

                let mut cursor = std::io::Cursor::new(buf);
                let mut logs = Vec::new();
                while cursor.remaining() > 0 {
                    let (timestamp, value) = decode_entry(&mut cursor)?;

                    let mut log = LogRecord::from(value);
                    let metadata = log.metadata_mut().value_mut();
                    metadata.insert(path!("fluent", "timestamp"), timestamp);
                    metadata.insert(path!("fluent", "tag"), tag.clone());

                    logs.push(log);
                }

                match option {
                    Some(options) => Ok(Some(ForwardFrame {
                        logs,
                        chunk: options.chunk,
                    })),
                    None => Ok(Some(ForwardFrame { logs, chunk: None })),
                }
            }

            // Message Mode
            //
            // https://github.com/fluent/fluentd/wiki/Forward-Protocol-Specification-v1#message-modes
            0xce => {
                // uint 32
                let secs = reader.read_u32()?;
                let ts = DateTime::from_timestamp(secs as i64, 0).ok_or(MsgPackError::Timestamp)?;

                let value = decode_value(reader)?;

                let mut log = LogRecord::from(value);
                let metadata = log.metadata_mut().value_mut();
                metadata.insert(path!("fluent", "timestamp"), ts);
                metadata.insert(path!("fluent", "tag"), tag);

                if len == 4 {
                    // only `size` or `chunk`
                    let options = decode_options(reader)?;
                    Ok(Some(ForwardFrame {
                        logs: vec![log],
                        chunk: options.chunk,
                    }))
                } else {
                    Ok(Some(ForwardFrame {
                        logs: vec![log],
                        chunk: None,
                    }))
                }
            }

            typ => Err(MsgPackError::UnknownType(typ, "entries").into()),
        }
    }
}

impl tokio_util::codec::Decoder for ForwardDecoder {
    type Item = (ForwardFrame, usize);
    type Error = DecodeError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        // fluent's Forward protocol do not have a length header, or something like that,
        // so we can't frame the input stream, and we can't tell if the buf have any message.
        //
        // The only way to figure out if we have got a whole message is parse/deserialize it,
        // That's the reason why we use Cursor here, and it may hurt performance too.
        let mut cursor = std::io::Cursor::new(&src[..]);
        match self.decode_internal(&mut cursor) {
            Ok(result) => {
                let size = cursor.position() as usize;
                src.advance(size);

                Ok(result.map(|frame| (frame, size)))
            }
            Err(err) => match err {
                DecodeError::IO(err) => {
                    if err.kind() == std::io::ErrorKind::UnexpectedEof {
                        Ok(None)
                    } else {
                        Err(DecodeError::IO(err))
                    }
                }
                _ => Err(err),
            },
        }
    }
}

struct ForwardAcker {
    chunks: Vec<Vec<u8>>,
}

impl TcpSourceAcker for ForwardAcker {
    fn build_ack(self, ack: TcpSourceAck) -> Option<Bytes> {
        if self.chunks.is_empty() {
            return None;
        }

        if TcpSourceAck::Ack != ack {
            return None;
        }

        let mut resp = Vec::new();
        for chunk in &self.chunks {
            resp.extend(encode_ack_resp(chunk));
        }

        Some(Bytes::from(resp))
    }
}

struct ForwardFrame {
    logs: Vec<LogRecord>,
    chunk: Option<Vec<u8>>,
}

impl From<ForwardFrame> for Events {
    fn from(frame: ForwardFrame) -> Self {
        Events::Logs(frame.logs)
    }
}

#[derive(Clone)]
struct ForwardSource;

impl TcpSource for ForwardSource {
    type Error = DecodeError;
    type Item = ForwardFrame;
    type Decoder = ForwardDecoder;
    type Acker = ForwardAcker;

    fn decoder(&self) -> Self::Decoder {
        ForwardDecoder
    }

    fn handle_events(&self, batch: &mut [Events], peer: SocketAddr, _size: usize) {
        for events in batch {
            events.for_each_log(|log| {
                log.metadata_mut()
                    .value_mut()
                    .insert("fluent.host", peer.ip().to_string());
            })
        }
    }

    fn build_acker(&self, items: &[Self::Item]) -> Self::Acker {
        let mut chunks = Vec::new();
        for frame in items {
            if let Some(chunk) = &frame.chunk {
                chunks.push(chunk.clone());
            }
        }

        ForwardAcker { chunks }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use bytes::BytesMut;
    use chrono::DateTime;
    use event::LogRecord;
    use tokio_util::codec::Decoder as _;
    use value::Value;

    use super::msgpack::decode_value;
    use super::{Config, ForwardDecoder, ForwardFrame, encode_ack_resp};

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }

    #[test]
    fn forward_without_option() {
        let input: [u8; 113] = [
            0x92, 0xa4, 0x74, 0x65, 0x73, 0x74, 0x91, 0xdd, 0x0, 0x0, 0x0, 0x2, 0xd7, 0x0, 0x67,
            0x8a, 0xbe, 0x12, 0x1f, 0xa8, 0xc8, 0xb8, 0x84, 0xa6, 0x6e, 0x75, 0x6c, 0x6c, 0x65,
            0x64, 0xc0, 0xa5, 0x62, 0x61, 0x73, 0x69, 0x63, 0xc3, 0xa4, 0x6c, 0x69, 0x73, 0x74,
            0x94, 0xc3, 0xc0, 0x93, 0xc3, 0xc0, 0xc3, 0x82, 0xa5, 0x62, 0x61, 0x73, 0x69, 0x63,
            0xc3, 0xa5, 0x62, 0x75, 0x64, 0x64, 0x79, 0xcb, 0x3f, 0xf1, 0x99, 0x99, 0x99, 0x99,
            0x99, 0x9a, 0xa3, 0x6d, 0x61, 0x70, 0x83, 0xa5, 0x62, 0x61, 0x73, 0x69, 0x63, 0xc3,
            0xa4, 0x6c, 0x69, 0x73, 0x74, 0x93, 0xc3, 0xc0, 0xc3, 0xa3, 0x6d, 0x61, 0x70, 0x82,
            0xa5, 0x62, 0x61, 0x73, 0x69, 0x63, 0xc3, 0xa5, 0x62, 0x75, 0x64, 0x64, 0x79, 0xff,
        ];

        let mut decoder = ForwardDecoder;
        let mut data = BytesMut::from(input.as_slice());

        let (ForwardFrame { logs, chunk }, size) = decoder.decode(&mut data).unwrap().unwrap();

        assert!(chunk.is_none());
        assert_eq!(size, input.len());
        assert_eq!(logs.len(), 1);

        let log = &logs[0];
        let metadata = log.metadata().value();
        let value = log.value();

        let timestamp = DateTime::parse_from_rfc3339("2025-01-17T20:31:14.531155128Z")
            .unwrap()
            .to_utc();
        assert_eq!(
            metadata,
            &value::value!({
                "fluent": {
                    "tag": "test",
                    "timestamp": timestamp,
                }
            })
        );
        assert_eq!(
            value,
            &value::value!({
              "nulled": null,
              "basic": true,
              "list": [
                true,
                null,
                [true, null, true],
                {
                  "basic": true,
                  "buddy": 1.1
                }
              ],
              "map": {
                "basic": true,
                "list": [true, null, true],
                "map": {
                  "basic": true,
                  "buddy": (-1)
                }
              }
            })
        );
    }

    #[test]
    fn fluentd_v1() {
        let input = [
            147, 168, 77, 114, 87, 120, 110, 79, 83, 120, 219, 0, 0, 0, 33, 146, 215, 0, 103, 141,
            58, 176, 16, 184, 227, 211, 129, 163, 107, 101, 121, 176, 54, 89, 110, 103, 76, 50, 48,
            88, 122, 104, 69, 85, 97, 71, 73, 68, 131, 164, 115, 105, 122, 101, 1, 170, 99, 111,
            109, 112, 114, 101, 115, 115, 101, 100, 164, 116, 101, 120, 116, 165, 99, 104, 117,
            110, 107, 185, 89, 115, 69, 114, 122, 69, 86, 69, 72, 84, 101, 107, 66, 101, 73, 88,
            78, 82, 106, 120, 84, 119, 61, 61, 10,
        ];

        let mut decoder = ForwardDecoder;
        let mut data = BytesMut::from(input.as_slice());

        decoder.decode(&mut data).unwrap();
    }

    fn mock_event(timestamp: &str, msg: &str) -> LogRecord {
        let mut value = Value::Object(Default::default());
        value.insert("message", msg);

        let mut log = LogRecord::from(value);
        let metadata = log.metadata_mut().value_mut();
        metadata.insert("fluent.tag", "tag.name");
        let timestamp = DateTime::parse_from_rfc3339(timestamp).unwrap().to_utc();
        metadata.insert("fluent.timestamp", timestamp);

        log
    }

    #[test]
    fn message_mode_without_option() {
        //[
        //  "tag.name",
        //  1441588984,
        //  {"message": "bar"},
        //]
        let input = vec![
            147u8, 168, 116, 97, 103, 46, 110, 97, 109, 101, 206, 85, 236, 230, 248, 129, 167, 109,
            101, 115, 115, 97, 103, 101, 163, 98, 97, 114,
        ];

        let mut decoder = ForwardDecoder;
        let mut data = BytesMut::from(input.as_slice());

        let (ForwardFrame { logs, chunk }, size) = decoder.decode(&mut data).unwrap().unwrap();

        assert_eq!(logs, vec![mock_event("2015-09-07T01:23:04Z", "bar")]);
        assert!(chunk.is_none());
        assert_eq!(size, input.len());
    }

    #[test]
    fn message_mode_with_option() {
        //[
        //  "tag.name",
        //   1441588984,
        //   { "message": "bar" },
        //   { "size": 1 }
        //]
        let input = vec![
            148, 168, 116, 97, 103, 46, 110, 97, 109, 101, 206, 85, 236, 230, 248, 129, 167, 109,
            101, 115, 115, 97, 103, 101, 163, 98, 97, 114, 129, 164, 115, 105, 122, 101, 1,
        ];

        let mut decoder = ForwardDecoder;
        let mut data = BytesMut::from(input.as_slice());

        let (ForwardFrame { logs, chunk }, size) = decoder.decode(&mut data).unwrap().unwrap();

        assert_eq!(logs, vec![mock_event("2015-09-07T01:23:04Z", "bar")]);
        assert!(chunk.is_none());
        assert_eq!(size, input.len());
    }

    #[test]
    fn forward_mode() {
        //[
        //    "tag.name",
        //    [
        //        [1441588984, {"message": "foo"}],
        //        [1441588985, {"message": "bar"}],
        //        [1441588986, {"message": "baz"}]
        //    ]
        //]
        let input = [
            146, 168, 116, 97, 103, 46, 110, 97, 109, 101, 147, 146, 206, 85, 236, 230, 248, 129,
            167, 109, 101, 115, 115, 97, 103, 101, 163, 102, 111, 111, 146, 206, 85, 236, 230, 249,
            129, 167, 109, 101, 115, 115, 97, 103, 101, 163, 98, 97, 114, 146, 206, 85, 236, 230,
            250, 129, 167, 109, 101, 115, 115, 97, 103, 101, 163, 98, 97, 122,
        ];

        let mut decoder = ForwardDecoder;
        let mut data = BytesMut::from(input.as_slice());

        let (ForwardFrame { logs, chunk }, size) = decoder.decode(&mut data).unwrap().unwrap();

        assert_eq!(
            logs,
            vec![
                mock_event("2015-09-07T01:23:04Z", "foo"),
                mock_event("2015-09-07T01:23:05Z", "bar"),
                mock_event("2015-09-07T01:23:06Z", "baz"),
            ]
        );
        assert!(chunk.is_none());
        assert_eq!(size, input.len());
    }

    #[test]
    fn forward_with_option() {
        //[
        //    "tag.name",
        //    [
        //        [1441588984, {"message": "foo"}],
        //        [1441588985, {"message": "bar"}],
        //        [1441588986, {"message": "baz"}]
        //    ]
        //]
        let input = [
            147, 168, 116, 97, 103, 46, 110, 97, 109, 101, 147, 146, 206, 85, 236, 230, 248, 129,
            167, 109, 101, 115, 115, 97, 103, 101, 163, 102, 111, 111, 146, 206, 85, 236, 230, 249,
            129, 167, 109, 101, 115, 115, 97, 103, 101, 163, 98, 97, 114, 146, 206, 85, 236, 230,
            250, 129, 167, 109, 101, 115, 115, 97, 103, 101, 163, 98, 97, 122, 129, 164, 115, 105,
            122, 101, 3,
        ];

        let mut decoder = ForwardDecoder;
        let mut data = BytesMut::from(input.as_slice());

        let (ForwardFrame { logs, chunk }, size) = decoder.decode(&mut data).unwrap().unwrap();

        assert_eq!(
            logs,
            vec![
                mock_event("2015-09-07T01:23:04Z", "foo"),
                mock_event("2015-09-07T01:23:05Z", "bar"),
                mock_event("2015-09-07T01:23:06Z", "baz"),
            ]
        );
        assert!(chunk.is_none());
        assert_eq!(size, input.len());
    }

    #[test]
    fn packed_forward_mode_without_option() {
        //[
        //    "tag.name",
        //    <packed messages>
        //]
        //
        //With packed messages as bin:
        // [1441588984, {"message": "foo"}]
        // [1441588985, {"message": "bar"}]
        // [1441588986, {"message": "baz"}]

        // message
        // compressed

        let input = [
            147, 168, 116, 97, 103, 46, 110, 97, 109, 101, 196, 57, 146, 206, 85, 236, 230, 248,
            129, 167, 109, 101, 115, 115, 97, 103, 101, 163, 102, 111, 111, 146, 206, 85, 236, 230,
            249, 129, 167, 109, 101, 115, 115, 97, 103, 101, 163, 98, 97, 114, 146, 206, 85, 236,
            230, 250, 129, 167, 109, 101, 115, 115, 97, 103, 101, 163, 98, 97, 122, 129,
            // fixstr
            //   c    o    m    p    r    e    s    s    e    d
            170, 99, 111, 109, 112, 114, 101, 115, 115, 101, 100, // foo
            163, 102, 111, 111,
        ];

        let mut decoder = ForwardDecoder;
        let mut data = BytesMut::from(input.as_slice());
        let (ForwardFrame { logs, chunk }, size) = decoder.decode(&mut data).unwrap().unwrap();

        assert_eq!(
            logs,
            vec![
                mock_event("2015-09-07T01:23:04Z", "foo"),
                mock_event("2015-09-07T01:23:05Z", "bar"),
                mock_event("2015-09-07T01:23:06Z", "baz"),
            ]
        );
        assert!(chunk.is_none());
        assert_eq!(size, input.len());
    }

    #[test]
    fn compressed_packed_forward() {
        //[
        //    "tag.name",
        //    <packed messages>,
        //    {"compressed": "gzip"}
        //]
        //
        //With gzip'd packed messages as bin:
        // [1441588984, {"message": "foo"}]
        // [1441588985, {"message": "bar"}]
        // [1441588986, {"message": "baz"}]
        let input = [
            147, 168, 116, 97, 103, 46, 110, 97, 109, 101, 196, 55, 31, 139, 8, 0, 245, 10, 168,
            96, 0, 3, 155, 116, 46, 244, 205, 179, 31, 141, 203, 115, 83, 139, 139, 19, 211, 83,
            23, 167, 229, 231, 79, 2, 9, 253, 68, 8, 37, 37, 22, 129, 133, 126, 33, 11, 85, 1, 0,
            53, 3, 158, 28, 57, 0, 0, 0, 129, 170, 99, 111, 109, 112, 114, 101, 115, 115, 101, 100,
            164, 103, 122, 105, 112,
        ];

        let mut decoder = ForwardDecoder;
        let mut data = BytesMut::from(input.as_slice());
        let (ForwardFrame { logs, chunk }, size) = decoder.decode(&mut data).unwrap().unwrap();

        assert_eq!(
            logs,
            vec![
                mock_event("2015-09-07T01:23:04Z", "foo"),
                mock_event("2015-09-07T01:23:05Z", "bar"),
                mock_event("2015-09-07T01:23:06Z", "baz"),
            ]
        );
        assert!(chunk.is_none());
        assert_eq!(size, input.len());
    }

    #[test]
    fn ack_resp() {
        let chunk = "2eeaa83113460dcdb3a8c8dc2a501fba";
        let got = encode_ack_resp(chunk.as_bytes());
        let want = [
            0x81, 0xa3, 0x61, 0x63, 0x6b, 0xd9, 0x20, 0x32, 0x65, 0x65, 0x61, 0x61, 0x38, 0x33,
            0x31, 0x31, 0x33, 0x34, 0x36, 0x30, 0x64, 0x63, 0x64, 0x62, 0x33, 0x61, 0x38, 0x63,
            0x38, 0x64, 0x63, 0x32, 0x61, 0x35, 0x30, 0x31, 0x66, 0x62, 0x61,
        ];
        let value = decode_value(&mut Cursor::new(&want)).unwrap();
        assert_eq!(
            value,
            value::value!({
                "ack": "2eeaa83113460dcdb3a8c8dc2a501fba"
            })
        );

        assert_eq!(got, want);
    }
}

#[cfg(all(test, feature = "fluent-integration-tests"))]
mod integration_tests {
    use bytes::Bytes;
    use event::{EventStatus, Events};
    use framework::Pipeline;
    use framework::config::ProxyConfig;
    use framework::http::HttpClient;
    use futures::Stream;
    use http::Request;
    use http_body_util::Full;
    use testify::container::Container;
    use testify::random::random_string;
    use testify::wait::wait_for_tcp;
    use testify::{collect_one, next_addr};
    use value::value;

    use super::*;

    use crate::testing::trace_init;

    const FLUENT_BIT_IMAGE: &str = "fluent/fluent-bit";
    const FLUENT_BIT_TAG: &str = "3.2.4";

    const FLUENTD_IMAGE: &str = "fluent/fluentd";
    const FLUENTD_TAG: &str = "v1.12";

    async fn run_fluent_bit(status: EventStatus) {
        trace_init();

        let (source_addr, receiver) = start_source(status).await;
        let source_port = source_addr.port();

        let config = format!(
            r#"
[SERVICE]
    Grace      0
    Flush      1
    Daemon     off

[INPUT]
    Name       http
    Host       0.0.0.0
    Port       8080

[OUTPUT]
    Name          forward
    Match         *
    Host          host.docker.internal
    Port          {source_port}
    Require_ack_response true
    "#,
        );
        let temp_dir = testify::temp_dir().join("fluent-bit.conf");
        std::fs::write(&temp_dir, &config).unwrap();

        let service_addr = next_addr();
        let tag = random_string(8);
        let value = random_string(16);
        let payload = format!(r#"{{ "key": "{value}" }}"#);

        let resp = Container::new(FLUENT_BIT_IMAGE, FLUENT_BIT_TAG)
            .with_volume(
                temp_dir.to_string_lossy(),
                "/fluent-bit/etc/fluent-bit.conf",
            )
            .with_tcp(8080, service_addr.port())
            .run(async move {
                let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
                let req = Request::post(format!("http://{service_addr}/{tag}"))
                    .header("content-type", "application/json")
                    .body(Full::new(Bytes::from(payload.clone())))
                    .unwrap();

                client.send(req).await.unwrap()
            })
            .await;

        assert_eq!(resp.status(), 201);

        let events = collect_one(receiver).await;

        assert_eq!(events.len(), 1);
        let log = events.into_logs().unwrap().remove(0);

        // metadata
        let metadata = log.metadata().value();
        assert!(metadata.contains("fluent"));
        assert!(metadata.contains("fluent.host"));
        assert!(metadata.contains("fluent.tag"));
        assert!(metadata.contains("fluent.timestamp"));

        assert_eq!(
            log.value(),
            &value!({
                "key": value
            })
        );
    }

    async fn start_source(status: EventStatus) -> (SocketAddr, impl Stream<Item = Events> + Unpin) {
        let (pipeline, recv) = Pipeline::new_test_finalize(status);
        let listen = next_addr();

        tokio::spawn(async move {
            let config = Config {
                listen,
                connection_limit: None,
                keepalive: None,
                receive_buffer: None,
                tls: None,
            };

            let source = config
                .build(SourceContext::new_test(pipeline))
                .await
                .unwrap();

            source.await.unwrap();
        });

        wait_for_tcp(listen).await;

        (listen, recv)
    }

    #[tokio::test]
    async fn fluent_bit() {
        run_fluent_bit(EventStatus::Delivered).await;
    }

    async fn run_fluentd(status: EventStatus, options: &str) {
        trace_init();

        let (source_addr, receiver) = start_source(status).await;
        let source_port = source_addr.port();
        let config = format!(
            r#"
<source>
  @type http
  bind 0.0.0.0
  port 8080
</source>

<match *>
  @type forward
  <server>
    name  local
    host  host.docker.internal
    port  {source_port}
  </server>
  <buffer>
    flush_mode immediate
  </buffer>
  require_ack_response true
  ack_response_timeout 1
  {options}
</match>
"#,
        );
        let temp_dir = testify::temp_dir().join("fluent.conf");
        std::fs::write(&temp_dir, &config).unwrap();

        let service_addr = next_addr();
        let tag = random_string(8);
        let value = random_string(16);
        let payload = format!(r#"{{ "key": "{value}" }}"#);

        let resp = Container::new(FLUENTD_IMAGE, FLUENTD_TAG)
            .with_volume(temp_dir.to_string_lossy(), "/fluentd/etc/fluent.conf")
            .with_tcp(8080, service_addr.port())
            .run(async move {
                let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
                let req = Request::post(format!("http://{service_addr}/{tag}"))
                    .header("content-type", "application/json")
                    .body(Full::new(Bytes::from(payload.clone())))
                    .unwrap();

                client.send(req).await.unwrap()
            })
            .await;

        assert_eq!(resp.status(), 200);

        let events = collect_one(receiver).await;

        assert_eq!(events.len(), 1);
        let log = events.into_logs().unwrap().remove(0);

        // metadata
        let metadata = log.metadata().value();
        assert!(metadata.contains("fluent"));
        assert!(metadata.contains("fluent.host"));
        assert!(metadata.contains("fluent.tag"));
        assert!(metadata.contains("fluent.timestamp"));

        assert_eq!(
            log.value(),
            &value!({
                "key": value
            })
        );
    }

    #[tokio::test]
    async fn fluentd() {
        run_fluentd(EventStatus::Delivered, "").await;
    }

    #[tokio::test]
    async fn fluentd_gzip() {
        run_fluentd(EventStatus::Delivered, "compress gzip").await;
    }

    #[tokio::test]
    async fn fluentd_rejection() {
        run_fluentd(EventStatus::Rejected, "").await;
    }
}
