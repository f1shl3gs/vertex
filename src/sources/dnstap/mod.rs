mod tcp;
mod unix;

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use bytes::{Buf, Bytes, BytesMut};
use chrono::DateTime;
use codecs::ReadyFrames;
use configurable::{configurable_component, Configurable};
use event::LogRecord;
use framework::config::{Output, Resource, SourceConfig, SourceContext};
use framework::{Pipeline, ShutdownSignal, Source};
use futures_util::StreamExt;
use proto::{Dnstap, Message, Policy};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_util::codec::Framed;
use value::Value;

mod proto {
    #![allow(clippy::trivially_copy_pass_by_ref)]

    include!(concat!(env!("OUT_DIR"), "/dnstap.rs"));
}

const fn default_max_frame_length() -> usize {
    128 * 1024
}

/// Listening mode for the `dnstap` source
#[derive(Configurable, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
enum Mode {
    /// Listen on TCP
    Tcp(tcp::Config),

    /// Listen on a Unix domain socket
    Unix(unix::Config),
}

#[configurable_component(source, name = "dnstap")]
struct Config {
    /// Maximum DNSTAP frame length that the source accepts.
    ///
    /// If any frame is longer than this, it is discarded.
    #[serde(default = "default_max_frame_length")]
    max_frame_length: usize,

    /// Whether to skip parsing or decoding of DNSTAP frames.
    ///
    /// If set to `true`, frames are not parsed or decoded. The raw frame data is
    /// set as a field on the event(called `rawData`) and encoded as a base64 string.
    raw_data_only: bool,

    #[serde(flatten)]
    mode: Mode,
}

#[async_trait::async_trait]
#[typetag::serde(name = "dnstap")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        match &self.mode {
            Mode::Tcp(config) => config.build(self.max_frame_length, cx).await,
            Mode::Unix(config) => config.build(self.max_frame_length, cx).await,
        }
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn resources(&self) -> Vec<Resource> {
        let resource = match &self.mode {
            Mode::Tcp(config) => config.resource(),
            Mode::Unix(config) => config.resource(),
        };

        vec![resource]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

#[rustfmt::skip]
const ACCEPT_FRAME: [u8; 42] = [
    // Zero represent next Frame is Control Frame
    0x00, 0x00, 0x00, 0x00,
    // Control Frame length
    0x00, 0x00, 0x00, 0x22,
    // Accept
    0x00, 0x00, 0x00, 0x01,
    // Control field content_type
    0x00, 0x00, 0x00, 0x01,
    // content field value len
    0x00, 0x00, 0x00, 0x16,
    // protobuf:dnstap.Dnstap
    0x70, 0x72, 0x6f, 0x74, 0x6f, 0x62, 0x75, 0x66, 0x3a, 0x64, 0x6e, 0x73, 0x74, 0x61, 0x70, 0x2e, 0x44, 0x6e, 0x73, 0x74, 0x61, 0x70
];

#[rustfmt::skip]
const FINISH_FRAME: [u8; 12] = [
    // Zero for next is Control Frame
    0x00, 0x00, 0x00, 0x00,
    // Control Frame length
    0x00, 0x00, 0x00, 0x04,
    // FINISH
    0x00, 0x00, 0x00, 0x05,
];

/// FStrm
///
/// https://github.com/farsightsec/fstrm
///
///       Client                            Server
///              Ready with ContentType
///           -------------------------------->
///              Accept with ContentType
///           <--------------------------------
///              Start with ContentType
///           -------------------------------->
///
///              Send Data 1
///           -------------------------------->
///              Send Data 2
///           -------------------------------->
///              Send Data 3
///           -------------------------------->
///
///              Stop Frame
///           -------------------------------->
///              Finish Frame
///           <--------------------------------
pub async fn serve_conn<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    bidirectional: bool,
    max_frame_length: usize,
    mut shutdown: ShutdownSignal,
    mut output: Pipeline,
) -> crate::Result<()> {
    // handshake
    if bidirectional {
        assert_frame(&mut stream, CONTROL_READY).await?;
        stream.write_all(&ACCEPT_FRAME).await?;
    }

    // read the Start ControlFrame
    assert_frame(&mut stream, CONTROL_START).await?;

    // reading data
    let reader = Framed::new(
        stream,
        LengthDelimitedCodec {
            limit: max_frame_length,
        },
    );
    let mut reader = ReadyFrames::new(reader);

    loop {
        let result = tokio::select! {
            result = reader.next() => result,
            _ = &mut shutdown => break,
        };

        match result {
            Some(Ok((frames, _size))) => {
                use prost::Message;

                let logs = frames
                    .into_iter()
                    .map(|mut frame| {
                        let tap = Dnstap::decode(&mut frame)?;
                        let value = tap_to_value(tap);
                        Ok::<LogRecord, prost::DecodeError>(LogRecord::from(value))
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                if let Err(_err) = output.send(logs).await {
                    return Ok(());
                }
            }
            Some(Err(err)) => match err {
                DecoderError::ControlFrame => break,
                DecoderError::LimitExceed(got) => {
                    warn!(
                        message = "frame size is exceeded the limit",
                        limit = max_frame_length,
                        ?got
                    );

                    return Ok(());
                }
                DecoderError::Io(err) => return Err(err.into()),
            },
            None => break,
        }
    }

    if bidirectional {
        let stream = reader.get_mut().get_mut();
        if let Err(err) = stream.write_all(&FINISH_FRAME).await {
            warn!(
                message = "Error writing acknowledgement, dropping connection",
                %err
            );
        }
    }

    Ok(())
}

// const CONTROL_ACCEPT: u32 = 0x01;
const CONTROL_START: u32 = 0x02;
// const CONTROL_STOP: u32 = 0x03;
const CONTROL_READY: u32 = 0x04;
// const CONTROL_FINISH: u32 = 0x05;

const CONTROL_FIELD_CONTENT_TYPE: u32 = 0x01;
const CONTROL_FRAME_LENGTH_MAX: usize = 512;

async fn assert_frame<R: AsyncRead + Unpin>(stream: &mut R, typ: u32) -> std::io::Result<()> {
    // decode zero
    let zero = stream.read_u32().await?;
    if zero != 0 {
        return Err(std::io::ErrorKind::InvalidData.into());
    }

    let mut len = stream.read_u32().await? as usize;
    if len > CONTROL_FRAME_LENGTH_MAX {
        return Err(std::io::ErrorKind::InvalidData.into());
    }

    let ct = stream.read_u32().await?;
    if typ != ct {
        return Err(std::io::ErrorKind::InvalidData.into());
    }

    len -= 4;
    let mut found = false;
    let mut buf = [0u8; 512];
    while len > 0 {
        let typ = stream.read_u32().await?;
        if typ != CONTROL_FIELD_CONTENT_TYPE {
            return Err(std::io::ErrorKind::InvalidData.into());
        }

        let field_len = stream.read_u32().await?;
        stream.read_exact(&mut buf[..field_len as usize]).await?;
        len -= field_len as usize + 4 + 4;

        if &buf[..field_len as usize] == b"protobuf:dnstap.Dnstap".as_ref() {
            found = true;
        }
    }

    if !found {
        return Err(std::io::ErrorKind::NotFound.into());
    }

    Ok(())
}

fn tap_to_value(tap: Dnstap) -> Value {
    let mut map = BTreeMap::new();

    if let Some(identity) = tap.identity {
        map.insert("identity".to_string(), Bytes::from(identity).into());
    }
    if let Some(version) = tap.version {
        map.insert("version".to_string(), Bytes::from(version).into());
    }
    if let Some(extra) = tap.extra {
        map.insert("extra".to_string(), Bytes::from(extra).into());
    }
    if tap.r#type == 1 {
        map.insert("type".to_string(), "Message".into());
    }

    if let Some(msg) = tap.message {
        map.insert("message".to_string(), tap_message_to_value(msg));
    }

    Value::Object(map)
}

fn tap_message_to_value(msg: Message) -> Value {
    let mut map = BTreeMap::new();

    let type_str = match msg.r#type {
        1 => Bytes::from("AuthQuery"),
        2 => Bytes::from("AuthResponse"),
        3 => Bytes::from("ResolverQuery"),
        4 => Bytes::from("ResolverResponse"),
        5 => Bytes::from("ClientQuery"),
        6 => Bytes::from("ClientResponse"),
        7 => Bytes::from("ForwarderQuery"),
        8 => Bytes::from("ForwarderResponse"),
        9 => Bytes::from("StubQuery"),
        10 => Bytes::from("StubResponse"),
        11 => Bytes::from("ToolQuery"),
        12 => Bytes::from("ToolResponse"),
        13 => Bytes::from("UpdateQuery"),
        14 => Bytes::from("UpdateResponse"),
        typ => Bytes::from(format!("Unknown dnstap message type: {typ}")),
    };
    map.insert("type".to_string(), type_str.into());

    if let Some(sf) = msg.socket_family {
        let sf = match sf {
            1 => "ipv4",
            2 => "ipv6",
            _ => "unknown",
        };

        map.insert("socket_family".to_string(), Bytes::from(sf).into());
    }

    if let Some(sp) = msg.socket_protocol {
        let sp = match sp {
            1 => "Udp",
            2 => "Tcp",
            3 => "Dot",
            4 => "Doh",
            5 => "DnsCryptUdp",
            6 => "DnsCryptTcp",
            7 => "Doq",
            _ => "Unknown",
        };

        map.insert("socket_protocol".to_string(), Bytes::from(sp).into());
    }

    if let Some(qa) = msg.query_address {
        match msg.socket_family {
            Some(1) => {
                let buf: [u8; 4] = qa[0..4].try_into().unwrap();
                let addr = IpAddr::V4(Ipv4Addr::from(buf));

                map.insert(
                    "query_address".to_string(),
                    Bytes::from(addr.to_string()).into(),
                );
            }
            Some(2) => {
                let buf: [u8; 16] = qa[0..16].try_into().unwrap();
                let addr = IpAddr::V6(Ipv6Addr::from(buf));
                map.insert(
                    "query_address".to_string(),
                    Bytes::from(addr.to_string()).into(),
                );
            }
            _ => {}
        }
    }

    if let Some(ra) = msg.response_address {
        match msg.socket_family {
            Some(1) => {
                let buf: [u8; 4] = ra[0..4].try_into().unwrap();
                let addr = IpAddr::V4(Ipv4Addr::from(buf));

                map.insert(
                    "response_address".to_string(),
                    Bytes::from(addr.to_string()).into(),
                );
            }
            Some(2) => {
                let buf: [u8; 16] = ra[0..16].try_into().unwrap();
                let addr = IpAddr::V6(Ipv6Addr::from(buf));
                map.insert(
                    "response_address".to_string(),
                    Bytes::from(addr.to_string()).into(),
                );
            }
            _ => {}
        }
    }

    if let Some(query_port) = msg.query_port {
        map.insert("query_port".to_string(), query_port.into());
    }
    if let Some(response_port) = msg.response_port {
        map.insert("response_port".to_string(), response_port.into());
    }

    // query time
    match (msg.query_time_sec, msg.query_time_nsec) {
        (Some(sec), Some(nsec)) => {
            let ts = DateTime::from_timestamp(sec as i64, nsec).unwrap().to_utc();
            map.insert("query_time".to_string(), ts.into());
        }
        (Some(sec), None) => {
            let ts = DateTime::from_timestamp(sec as i64, 0).unwrap().to_utc();
            map.insert("query_time".to_string(), ts.into());
        }
        _ => {}
    };

    // TODO: query message

    if let Some(query_zone) = msg.query_zone {
        map.insert("query_zone".to_string(), Bytes::from(query_zone).into());
    }

    // response time
    match (msg.response_time_sec, msg.response_time_nsec) {
        (Some(sec), Some(nsec)) => {
            let ts = DateTime::from_timestamp(sec as i64, nsec).unwrap().to_utc();
            map.insert("response_time".to_string(), ts.into());
        }
        (Some(sec), None) => {
            let ts = DateTime::from_timestamp(sec as i64, 0).unwrap().to_utc();
            map.insert("response_time".to_string(), ts.into());
        }
        _ => {}
    };

    // TODO: response message

    if let Some(policy) = msg.policy {
        map.insert("policy".to_string(), policy_to_value(policy));
    }

    if let Some(hp) = msg.http_protocol {
        let http_protocol = match hp {
            1 => "HTTP1",
            2 => "HTTP2",
            3 => "HTTP3",
            _ => "Unknown",
        };

        map.insert(
            "http_protocol".to_string(),
            Bytes::from(http_protocol).into(),
        );
    }

    Value::Object(map)
}

fn policy_to_value(policy: Policy) -> Value {
    let mut map = BTreeMap::new();

    if let Some(typ) = policy.r#type {
        map.insert("type".to_string(), Bytes::from(typ).into());
    }

    if let Some(rule) = policy.rule {
        map.insert("rule".to_string(), Bytes::from(rule).into());
    }

    if let Some(action) = policy.action {
        let action = match action {
            1 => "NXDOMAIN",
            2 => "NODATA",
            3 => "PASS",
            4 => "DROP",
            5 => "TRUNCATE",
            6 => "LOCAL_DATA",
            _ => "Unknown",
        };

        map.insert("action".to_string(), Bytes::from(action).into());
    }

    if let Some(m) = policy.r#match {
        let m = match m {
            1 => "QNAME",
            2 => "CLIENT_IP",
            3 => "RESPONSE_IP",
            4 => "NS_NAME",
            5 => "NS_IP",
            _ => "Unknown",
        };

        map.insert("match".to_string(), Bytes::from(m).into());
    }

    if let Some(value) = policy.value {
        map.insert("value".to_string(), Bytes::from(value).into());
    }

    Value::Object(map)
}

enum DecoderError {
    LimitExceed(usize),
    ControlFrame,
    Io(std::io::Error),
}

impl From<std::io::Error> for DecoderError {
    fn from(err: std::io::Error) -> Self {
        DecoderError::Io(err)
    }
}

struct LengthDelimitedCodec {
    limit: usize,
}

impl tokio_util::codec::Decoder for LengthDelimitedCodec {
    type Item = (Bytes, usize);
    type Error = DecoderError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }

        let len = u32::from_be_bytes(src[..4].try_into().unwrap()) as usize;
        if len == 0 {
            return Err(DecoderError::ControlFrame);
        }

        if self.limit < len {
            return Err(DecoderError::LimitExceed(len));
        }

        if src.len() <= len + 4 {
            return Ok(None);
        }

        src.advance(4);
        let frame = src.split_to(len).freeze();

        Ok(Some((frame, len)))
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.decode(src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
