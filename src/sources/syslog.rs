use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use bytes::Bytes;
use chrono::Utc;
use codecs::decoding::{
    BytesDeserializerConfig, DecodeError, OctetCountingDecoder, SyslogDeserializer,
};
use codecs::Decoder;
use configurable::{configurable_component, Configurable};
use event::Event;
use framework::config::Output;
use framework::config::{DataType, Resource, SourceConfig, SourceContext};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::source::util::{
    build_unix_stream_source, SocketListenAddr, TcpNullAcker, TcpSource,
};
use framework::tcp::TcpKeepaliveConfig;
use framework::tls::{MaybeTlsSettings, TlsConfig};
use framework::Source;
use futures_util::StreamExt;
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tokio::net::UdpSocket;
use tokio_util::udp::UdpFramed;

// The default max length of the input buffer
pub const fn default_max_length() -> usize {
    128 * 1024
}

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Mode {
    Tcp {
        /// The address to listen for connections on, or systemd#N to use the Nth
        /// socket passed by systemd socket activation. If an address is used it
        /// must include a port.
        #[configurable(required, format = "ip-address", example = "0.0.0.0:9000")]
        address: SocketListenAddr,

        /// Configures the TCP keepalive behavior for the connection to the source.
        keepalive: Option<TcpKeepaliveConfig>,

        /// Configures the TLS options for incoming connections.
        tls: Option<TlsConfig>,

        /// Configures the recive buffer size using the "SO_RCVBUF" option on the socket.
        #[serde(default, with = "humanize::bytes::serde_option")]
        receive_buffer_bytes: Option<usize>,

        /// The max number of TCP connections that will be processed.
        connection_limit: Option<u32>,
    },
    Udp {
        /// The address to listen for connections on, or systemd#N to use the Nth
        /// socket passed by systemd socket activation. If an address is used it
        /// must include a port
        #[configurable(required, format = "ip-address", example = "0.0.0.0:9000")]
        address: SocketAddr,

        /// Configures the recive buffer size using the "SO_RCVBUF" option on the socket.
        #[serde(default, with = "humanize::bytes::serde_option")]
        receive_buffer_bytes: Option<usize>,
    },
    #[cfg(unix)]
    Unix {
        /// Unix socket file path.
        #[configurable(required)]
        path: PathBuf,
    },
}

#[configurable_component(source, name = "syslog")]
#[derive(Debug)]
pub struct SyslogConfig {
    /// The type of socket to use.
    #[serde(flatten)]
    pub mode: Mode,

    /// The maximum buffer size of incoming messages. Messages larger than
    /// this are truncated.
    #[serde(default = "default_max_length")]
    pub max_length: usize,

    /// The key name added to each event representing the current host. This can
    /// be globally set via the global "host_key" option.
    ///
    /// The host key of the log. This differs from `hostname`
    pub host_key: Option<String>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "syslog")]
impl SourceConfig for SyslogConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let host_key = self
            .host_key
            .clone()
            .unwrap_or_else(|| log_schema().host_key().to_string());

        match self.mode.clone() {
            Mode::Tcp {
                address,
                keepalive,
                tls,
                receive_buffer_bytes,
                connection_limit,
            } => {
                let source = SyslogTcpSource {
                    max_length: self.max_length,
                    host_key,
                };
                let tls = MaybeTlsSettings::from_config(&tls, true)?;
                let shutdown_timeout = Duration::from_secs(30);

                source.run(
                    address,
                    keepalive,
                    shutdown_timeout,
                    tls,
                    receive_buffer_bytes,
                    cx,
                    false,
                    connection_limit,
                )
            }

            Mode::Udp {
                address,
                receive_buffer_bytes,
            } => Ok(udp(
                address,
                self.max_length,
                host_key,
                receive_buffer_bytes,
                cx.shutdown,
                cx.output,
            )),

            #[cfg(unix)]
            Mode::Unix { path } => {
                let decoder = Decoder::new(
                    OctetCountingDecoder::new_with_max_length(self.max_length).into(),
                    SyslogDeserializer.into(),
                );

                Ok(build_unix_stream_source(
                    path,
                    decoder,
                    move |events, host, byte_size| {
                        handle_events(events, &host_key, host, byte_size)
                    },
                    cx.shutdown,
                    cx.output,
                ))
            }
        }
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn resources(&self) -> Vec<Resource> {
        match self.mode.clone() {
            Mode::Tcp { address, .. } => vec![address.into()],
            Mode::Udp { address, .. } => vec![Resource::udp(address)],
            #[cfg(unix)]
            Mode::Unix { .. } => vec![],
        }
    }
}

#[derive(Debug, Clone)]
struct SyslogTcpSource {
    max_length: usize,
    host_key: String,
}

impl TcpSource for SyslogTcpSource {
    type Error = DecodeError;
    type Item = SmallVec<[Event; 1]>;
    type Decoder = Decoder;
    type Acker = TcpNullAcker;

    fn decoder(&self) -> Self::Decoder {
        Decoder::new(
            OctetCountingDecoder::new_with_max_length(self.max_length).into(),
            SyslogDeserializer.into(),
        )
    }

    fn handle_events(&self, events: &mut [Event], host: Bytes, size: usize) {
        handle_events(events, &self.host_key, Some(host), size)
    }

    fn build_acker(&self, _item: &[Self::Item]) -> Self::Acker {
        TcpNullAcker
    }
}

pub fn udp(
    addr: SocketAddr,
    _max_length: usize,
    host_key: String,
    receive_buffer_bytes: Option<usize>,
    shutdown: ShutdownSignal,
    mut output: Pipeline,
) -> framework::Source {
    Box::pin(async move {
        let socket = UdpSocket::bind(&addr)
            .await
            .expect("Failed to bind to UDP listener socket");

        if let Some(receive_buffer_bytes) = receive_buffer_bytes {
            if let Err(err) = framework::udp::set_receive_buffer_size(&socket, receive_buffer_bytes)
            {
                warn!(
                    message = "Failed configure receive buffer size on UDP socket",
                    %err
                );
            }
        }

        info!(
            message = "listening",
            %addr,
            r#type = "udp"
        );

        let mut stream = UdpFramed::new(
            socket,
            Decoder::new(
                BytesDeserializerConfig::new().into(),
                SyslogDeserializer.into(),
            ),
        )
        .take_until(shutdown)
        .filter_map(|frame| {
            let host_key = host_key.clone();
            async move {
                match frame {
                    Ok(((mut events, byte_size), received_from)) => {
                        let received_from = received_from.ip().to_string().into();
                        handle_events(&mut events, &host_key, Some(received_from), byte_size);
                        Some(events.remove(0))
                    }
                    Err(err) => {
                        warn!(
                            message = "Error reading datagram",
                            ?err,
                            internal_log_rate_limit = true
                        );

                        None
                    }
                }
            }
        })
        .boxed();

        match output.send_event_stream(&mut stream).await {
            Ok(()) => {
                info!(message = "Finished sending");
                Ok(())
            }
            Err(err) => {
                error!(
                    message = "Error sending line",
                    %err
                );

                Err(())
            }
        }
    })
}

fn handle_events(
    events: &mut [Event],
    host_key: &str,
    default_host: Option<Bytes>,
    _byte_size: usize,
) {
    // TODO: handle the byte_size

    for event in events {
        enrich_syslog_event(event, host_key, default_host.clone());
    }
}

fn enrich_syslog_event(event: &mut Event, host_key: &str, default_host: Option<Bytes>) {
    let log = event.as_mut_log();

    log.insert_field(log_schema().source_type_key(), "syslog");

    if let Some(default_host) = &default_host {
        log.insert_field("source_ip", default_host.clone());
    }

    let parsed_hostname = log
        .get_field("hostname")
        .map(|hostname| hostname.as_bytes());
    if let Some(parsed_host) = parsed_hostname.or(default_host) {
        log.insert_field(host_key, parsed_host);
    }

    let timestamp = log
        .get_field("timestamp")
        .and_then(|timestamp| timestamp.as_timestamp().cloned())
        .unwrap_or_else(Utc::now);
    log.insert_field(log_schema().timestamp_key(), timestamp);

    trace!(
        message = "Processing one event.",
        event = ?event
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Datelike, NaiveDate, TimeZone};
    use codecs::decoding::format::Deserializer;
    use event::log::Value;
    use event::{assert_event_data_eq, fields, LogRecord};

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<SyslogConfig>();
    }

    #[test]
    fn config_tcp() {
        let text = r#"
mode: tcp
address: 127.0.0.1:12345
"#;
        let config: SyslogConfig = serde_yaml::from_str(text).unwrap();

        assert!(matches!(config.mode, Mode::Tcp { .. }));
    }

    #[test]
    fn config_tcp_with_receive_buffer_size() {
        let config: SyslogConfig =
            serde_yaml::from_str("mode: tcp\naddress: 127.0.0.1:12345\nreceive_buffer_bytes: 1ki")
                .unwrap();

        let receive_buffer_bytes = match config.mode {
            Mode::Tcp {
                receive_buffer_bytes,
                ..
            } => receive_buffer_bytes,
            _ => unreachable!(),
        };

        assert_eq!(receive_buffer_bytes, Some(1024usize));
    }

    #[test]
    fn config_tcp_with_keepalive() {
        let config: SyslogConfig = serde_yaml::from_str(
            "mode: tcp\naddress: 127.0.0.1:12345\nkeepalive:\n  timeout: 120s",
        )
        .unwrap();

        match config.mode {
            Mode::Tcp { keepalive, .. } => {
                let keepalive = keepalive.unwrap();
                assert_eq!(keepalive.timeout, Some(Duration::from_secs(120)));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn config_udp() {
        let config: SyslogConfig =
            serde_yaml::from_str("mode: udp\naddress: 127.0.0.1:12345\nmax_length: 1024").unwrap();

        assert_eq!(config.max_length, 1024);

        match config.mode {
            Mode::Udp { address, .. } => {
                assert_eq!(address.to_string(), "127.0.0.1:12345".to_string());
            }
            _ => unreachable!(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn config_unix() {
        let config: SyslogConfig =
            serde_yaml::from_str("mode: unix\npath: /some/path/to/your.sock").unwrap();

        match config.mode {
            Mode::Unix { path } => {
                assert_eq!(path, PathBuf::from("/some/path/to/your.sock"));
            }

            _ => unreachable!(),
        }
    }

    fn event_from_bytes(
        host_key: &str,
        default_host: Option<Bytes>,
        bytes: Bytes,
    ) -> Option<Event> {
        let byte_size = bytes.len();
        let parser = SyslogDeserializer;
        let mut events = parser.parse(bytes).ok()?;
        handle_events(&mut events, host_key, default_host, byte_size);
        Some(events.remove(0))
    }

    #[test]
    fn syslog_ng_network_syslog_protocol() {
        // this should also match rsyslog omfwd with template=RSYSLOG_SyslogProtocol23Format
        let msg = "i am foobar";
        let raw = format!(
            r#"<13>1 2019-02-13T19:48:34+00:00 74794bfb6795 root 8449 - {}{} {}"#,
            r#"[meta sequenceId="1" sysUpTime="37" language="EN"]"#,
            r#"[origin ip="192.168.0.1" software="test"]"#,
            msg
        );

        let log = LogRecord::from(fields!(
            log_schema().message_key() => msg,
            log_schema().timestamp_key() => chrono::Utc.with_ymd_and_hms(2019, 2, 13, 19, 48, 34).unwrap(),
            log_schema().source_type_key() => "syslog",
            "host" => "74794bfb6795",
            "hostname" => "74794bfb6795",
            "meta" => fields!(
                "sequenceId" => "1",
                "sysUpTime" => "37",
                "language" => "EN",
            ),
            "origin" => fields!(
                "software" => "test",
                "ip" => "192.168.0.1",
            ),
            "severity" => "notice",
            "facility" => "user",
            "version" => 1,
            "appname" => "root",
            "procid" => 8449,
        ));

        let want = Event::from(log);
        assert_event_data_eq!(event_from_bytes("host", None, raw.into()).unwrap(), want);
    }

    #[test]
    fn handles_incorrect_sd_element() {
        let msg = "qwerty";
        let raw = format!(
            r#"<13>1 2019-02-13T19:48:34+00:00 74794bfb6795 root 8449 - {} {}"#,
            r#"[incorrect x]"#, msg
        );

        let event = event_from_bytes("host", None, raw.into()).unwrap();

        let want = Event::from(LogRecord::from(fields!(
            log_schema().timestamp_key() => chrono::Utc.with_ymd_and_hms(2019, 2, 13, 19, 48, 34).unwrap(),
            log_schema().host_key() => "74794bfb6795",
            log_schema().source_type_key() => "syslog",
            "hostname" => "74794bfb6795",
            "severity" => "notice",
            "facility" => "user",
            "version" => 1,
            "appname" => "root",
            "procid" => 8449,
            log_schema().message_key() => msg,
        )));

        assert_event_data_eq!(event, want);

        let raw = format!(
            r#"<13>1 2019-02-13T19:48:34+00:00 74794bfb6795 root 8449 - {} {}"#,
            r#"[incorrect x=]"#, msg
        );

        let event = event_from_bytes("host", None, raw.into()).unwrap();
        assert_event_data_eq!(event, want);
    }

    #[test]
    fn handles_empty_sd_element() {
        fn there_is_map_called_empty(event: Event) -> bool {
            let value = event.as_log().get_field("empty").expect("empty exists");

            matches!(value, Value::Object(_))
        }

        for (input, want) in [
            (
                format!(
                    r#"<13>1 2019-02-13T19:48:34+00:00 74794bfb6795 root 8449 - {} qwerty"#,
                    r#"[empty]"#
                ),
                true,
            ),
            (
                format!(
                    r#"<13>1 2019-02-13T19:48:34+00:00 74794bfb6795 root 8449 - {} qwerty"#,
                    r#"[non_empty x="1"][empty]"#
                ),
                true,
            ),
            (
                format!(
                    r#"<13>1 2019-02-13T19:48:34+00:00 74794bfb6795 root 8449 - {} qwerty"#,
                    r#"[empty][non_empty x="1"]"#
                ),
                true,
            ),
            (
                format!(
                    r#"<13>1 2019-02-13T19:48:34+00:00 74794bfb6795 root 8449 - {} qwerty"#,
                    r#"[empty not_really="testing the test"]"#
                ),
                true,
            ),
        ] {
            let event = event_from_bytes("host", None, input.clone().into()).unwrap();
            assert_eq!(there_is_map_called_empty(event), want, "input: {}", input);
        }
    }

    #[test]
    fn handle_weired_whitespace() {
        // this should also match rsyslog omfwd with template=RSYSLOG_SyslogProtocol23Format
        let raw = r#"
            <13>1 2019-02-13T19:48:34+00:00 74794bfb6795 root 8449 - [meta sequenceId="1"] i am foobar
            "#;
        let cleaned = r#"<13>1 2019-02-13T19:48:34+00:00 74794bfb6795 root 8449 - [meta sequenceId="1"] i am foobar"#;

        assert_event_data_eq!(
            event_from_bytes("host", None, raw.into()).unwrap(),
            event_from_bytes("host", None, cleaned.into()).unwrap()
        );
    }

    #[test]
    fn syslog_ng_default_network() {
        let msg = "i am foobar";
        let raw = format!(r#"<13>Feb 13 20:07:26 74794bfb6795 root[8539]: {}"#, msg);
        let event = event_from_bytes("host", None, raw.into()).unwrap();

        let value = event
            .as_log()
            .get_field(log_schema().timestamp_key())
            .unwrap();
        let year = value.as_timestamp().unwrap().naive_local().year();
        let date: DateTime<Utc> = chrono::Local
            .with_ymd_and_hms(year, 2, 13, 20, 7, 26)
            .unwrap()
            .into();

        let want: Event = LogRecord::from(fields!(
            log_schema().timestamp_key() => date,
            log_schema().host_key() => "74794bfb6795",
            log_schema().source_type_key() => "syslog",
            "hostname" => "74794bfb6795",
            "severity" => "notice",
            "facility" => "user",
            "appname" => "root",
            "procid" => 8539,
            log_schema().message_key() => msg,
        ))
        .into();

        assert_event_data_eq!(event, want);
    }

    #[test]
    fn rsyslog_omfwd_tcp_default() {
        let msg = "start";
        let raw = format!(
            r#"<190>Feb 13 21:31:56 74794bfb6795 liblogging-stdlog:  [origin software="rsyslogd" swVersion="8.24.0" x-pid="8979" x-info="http://www.rsyslog.com"] {}"#,
            msg
        );
        let event = event_from_bytes("host", None, raw.into()).unwrap();

        let value = event
            .as_log()
            .get_field(log_schema().timestamp_key())
            .unwrap();
        let year = value.as_timestamp().unwrap().naive_local().year();
        let date: DateTime<Utc> = chrono::Local
            .with_ymd_and_hms(year, 2, 13, 21, 31, 56)
            .unwrap()
            .into();
        let want: Event = LogRecord::from(fields!(
            log_schema().timestamp_key() => date,
            log_schema().message_key() => msg,
            log_schema().source_type_key() => "syslog",
            "host" => "74794bfb6795",
            "hostname" => "74794bfb6795",
            "severity" => "info",
            "facility" => "local7",
            "appname" => "liblogging-stdlog",
            "origin" => fields!(
                "software" => "rsyslogd",
                "swVersion" => "8.24.0",
                "x-pid" => "8979",
                "x-info" => "http://www.rsyslog.com",
            )
        ))
        .into();

        assert_event_data_eq!(event, want);
    }

    #[test]
    fn rsyslog_omfwd_tcp_forward_format() {
        let msg = "start";
        let raw = format!(
            r#"<190>2019-02-13T21:53:30.605850+00:00 74794bfb6795 liblogging-stdlog:  [origin software="rsyslogd" swVersion="8.24.0" x-pid="9043" x-info="http://www.rsyslog.com"] {}"#,
            msg
        );
        let event = event_from_bytes("host", None, raw.into()).unwrap();

        let dt = NaiveDate::from_ymd_opt(2019, 2, 13)
            .unwrap()
            .and_hms_micro_opt(21, 53, 30, 605_850)
            .unwrap();

        let want: Event = LogRecord::from(fields!(
            log_schema().timestamp_key() => Utc.from_utc_datetime(&dt),
            log_schema().message_key() => msg,
            log_schema().source_type_key() => "syslog",
            "host" => "74794bfb6795",
            "hostname" => "74794bfb6795",
            "severity" => "info",
            "facility" => "local7",
            "appname" => "liblogging-stdlog",
            "origin" => fields!(
                "software" => "rsyslogd",
                "swVersion" => "8.24.0",
                "x-pid" => "9043",
                "x-info" => "http://www.rsyslog.com",
            )
        ))
        .into();

        assert_event_data_eq!(event, want);
    }
}
