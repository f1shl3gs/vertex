use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use codecs::Decoder;
use codecs::decoding::{
    BytesDeserializerDecoder, DecodeError, OctetCountingDecoder, SyslogDeserializer,
};
use configurable::{Configurable, configurable_component};
use event::log::OwnedValuePath;
use event::log::path::PathPrefix;
use event::{Events, LogRecord, event_path};
use framework::Source;
use framework::config::{Output, Resource, SourceConfig, SourceContext};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::source::tcp::{SocketListenAddr, TcpNullAcker, TcpSource};
use framework::source::unix::build_unix_stream_source;
use framework::tcp::TcpKeepaliveConfig;
use framework::tls::TlsConfig;
use futures_util::StreamExt;
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
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
        connection_limit: Option<usize>,
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

/// This source allows to collect Syslog messages through a Unix socket server (UDP or
/// TCP) or over the network using TCP or UDP.
#[configurable_component(source, name = "syslog")]
pub struct Config {
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
    pub host_key: Option<OwnedValuePath>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "syslog")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let host_key = self
            .host_key
            .clone()
            .unwrap_or_else(|| log_schema().host_key().path.clone());

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
                let shutdown_timeout = Duration::from_secs(30);

                source.run(
                    address,
                    keepalive,
                    shutdown_timeout,
                    tls.as_ref(),
                    receive_buffer_bytes,
                    cx,
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
                    SyslogDeserializer::default().into(),
                );

                build_unix_stream_source(
                    path,
                    decoder,
                    move |events, _received_from| handle_events(events, &host_key, None),
                    cx.shutdown,
                    cx.output,
                )
            }
        }
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn resources(&self) -> Vec<Resource> {
        match self.mode.clone() {
            Mode::Tcp { address, .. } => vec![address.into()],
            Mode::Udp { address, .. } => vec![Resource::udp(address)],
            #[cfg(unix)]
            Mode::Unix { .. } => vec![],
        }
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
struct SyslogTcpSource {
    max_length: usize,
    host_key: OwnedValuePath,
}

impl TcpSource for SyslogTcpSource {
    type Error = DecodeError;
    type Item = Events;
    type Decoder = Decoder;
    type Acker = TcpNullAcker;

    fn decoder(&self) -> Self::Decoder {
        Decoder::new(
            OctetCountingDecoder::new_with_max_length(self.max_length).into(),
            SyslogDeserializer::default().into(),
        )
    }

    fn handle_events(&self, batch: &mut [Events], peer: SocketAddr, _size: usize) {
        let default_host = Some(peer);

        for events in batch {
            handle_events(events, &self.host_key, default_host)
        }
    }

    fn build_acker(&self, _item: &[Self::Item]) -> Self::Acker {
        TcpNullAcker
    }
}

fn udp(
    addr: SocketAddr,
    _max_length: usize,
    host_key: OwnedValuePath,
    receive_buffer_bytes: Option<usize>,
    shutdown: ShutdownSignal,
    mut output: Pipeline,
) -> Source {
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
                BytesDeserializerDecoder::new().into(),
                SyslogDeserializer::default().into(),
            ),
        )
        .take_until(shutdown)
        .filter_map(|frame| {
            let host_key = host_key.clone();

            async move {
                match frame {
                    Ok(((mut events, _byte_size), received_from)) => {
                        handle_events(&mut events, &host_key, Some(received_from));
                        Some(events)
                    }
                    Err(err) => {
                        warn!(
                            message = "Error reading datagram",
                            %err,
                            internal_log_rate_limit = true
                        );

                        None
                    }
                }
            }
        })
        .boxed();

        match output.send_stream(&mut stream).await {
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

#[inline]
fn handle_events(events: &mut Events, host_key: &OwnedValuePath, default_host: Option<SocketAddr>) {
    // TODO: handle the byte_size
    events.for_each_log(|log| {
        enrich_syslog_log(log, host_key, default_host);
    })
}

fn enrich_syslog_log(
    log: &mut LogRecord,
    host_key: &OwnedValuePath,
    default_host: Option<SocketAddr>,
) {
    log.insert(log_schema().source_type_key(), "syslog");

    if let Some(default_host) = &default_host {
        log.insert(event_path!("source_ip"), default_host.ip().to_string());
    }

    let parsed_hostname = log
        .get(event_path!("hostname"))
        .map(|hostname| hostname.coerce_to_bytes());
    if let Some(parsed_host) = parsed_hostname {
        log.insert((PathPrefix::Event, host_key), parsed_host);
    } else if let Some(default_host) = &default_host {
        log.insert((PathPrefix::Event, host_key), default_host.ip().to_string());
    }

    let timestamp = log
        .get(event_path!("timestamp"))
        .and_then(|timestamp| timestamp.as_timestamp().cloned())
        .unwrap_or_else(Utc::now);
    log.insert(log_schema().timestamp_key(), timestamp);
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use chrono::{DateTime, Datelike, NaiveDate, TimeZone};
    use codecs::decoding::format::Deserializer;
    use event::LogRecord;
    use event::log::{Value, parse_value_path};
    use testify::assert_event_data_eq;
    use value::value;

    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    #[test]
    fn config_tcp() {
        let text = r#"
mode: tcp
address: 127.0.0.1:12345
"#;
        let config: Config = serde_yaml::from_str(text).unwrap();

        assert!(matches!(config.mode, Mode::Tcp { .. }));
    }

    #[test]
    fn config_tcp_with_receive_buffer_size() {
        let config: Config =
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
        let config: Config = serde_yaml::from_str(
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
        let config: Config =
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
        let config: Config =
            serde_yaml::from_str("mode: unix\npath: /some/path/to/your.sock").unwrap();

        match config.mode {
            Mode::Unix { path } => {
                assert_eq!(path, PathBuf::from("/some/path/to/your.sock"));
            }

            _ => unreachable!(),
        }
    }

    fn event_from_bytes(host_key: &str, bytes: Bytes) -> Option<LogRecord> {
        let parser = SyslogDeserializer::default();
        let mut events = parser.parse(bytes).ok()?;
        let host_key = parse_value_path(host_key).unwrap();
        handle_events(&mut events, &host_key, None);

        let log = events.into_logs().unwrap().remove(0);

        Some(log)
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

        let want = {
            let mut log = LogRecord::from(value!({
                "host": "74794bfb6795",
                "hostname": "74794bfb6795",
                "meta": {
                    "sequenceId": "1",
                    "sysUpTime": "37",
                    "language": "EN",
                },
                "origin": {
                    "software": "test",
                    "ip": "192.168.0.1",
                },
                "severity": "notice",
                "facility": "user",
                "version": 1,
                "appname": "root",
                "procid": 8449,
            }));

            log.insert(log_schema().message_key(), msg);
            log.insert(
                log_schema().timestamp_key(),
                Utc.with_ymd_and_hms(2019, 2, 13, 19, 48, 34).unwrap(),
            );
            log.insert(log_schema().source_type_key(), "syslog");

            log
        };

        assert_event_data_eq!(event_from_bytes("host", raw.into()).unwrap(), want);
    }

    #[test]
    fn handles_incorrect_sd_element() {
        let msg = "qwerty";
        let raw = format!(
            r#"<13>1 2019-02-13T19:48:34+00:00 74794bfb6795 root 8449 - {} {}"#,
            r#"[incorrect x]"#, msg
        );

        let event = event_from_bytes("host", raw.into()).unwrap();
        let want = {
            let mut log = LogRecord::from(msg);

            log.insert(
                log_schema().timestamp_key(),
                Utc.with_ymd_and_hms(2019, 2, 13, 19, 48, 34).unwrap(),
            );
            log.insert(log_schema().host_key(), "74794bfb6795");
            log.insert(log_schema().source_type_key(), "syslog");
            log.insert("hostname", "74794bfb6795");
            log.insert("severity", "notice");
            log.insert("facility", "user");
            log.insert("version", 1);
            log.insert("appname", "root");
            log.insert("procid", 8449);

            log
        };

        assert_event_data_eq!(event, want);

        let raw = format!(
            r#"<13>1 2019-02-13T19:48:34+00:00 74794bfb6795 root 8449 - {} {}"#,
            r#"[incorrect x=]"#, msg
        );

        let event = event_from_bytes("host", raw.into()).unwrap();
        assert_event_data_eq!(event, want);
    }

    #[test]
    fn handles_empty_sd_element() {
        fn there_is_map_called_empty(log: LogRecord) -> bool {
            let value = log.get("empty").expect("empty exists");

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
            let event = event_from_bytes("host", input.clone().into()).unwrap();
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
            event_from_bytes("host", raw.into()).unwrap(),
            event_from_bytes("host", cleaned.into()).unwrap()
        );
    }

    #[test]
    fn syslog_ng_default_network() {
        let msg = "i am foobar";
        let raw = format!(r#"<13>Feb 13 20:07:26 74794bfb6795 root[8539]: {}"#, msg);
        let log = event_from_bytes("host", raw.into()).unwrap();

        let value = log.get(log_schema().timestamp_key()).unwrap();
        let year = value.as_timestamp().unwrap().naive_local().year();
        let date: DateTime<Utc> = chrono::Local
            .with_ymd_and_hms(year, 2, 13, 20, 7, 26)
            .unwrap()
            .into();

        let want = {
            let mut log = LogRecord::from(value!({
                "hostname": "74794bfb6795",
                "severity": "notice",
                "facility": "user",
                "appname": "root",
                "procid": 8539,
            }));

            log.insert(log_schema().timestamp_key(), date);
            log.insert(log_schema().host_key(), "74794bfb6795");
            log.insert(log_schema().source_type_key(), "syslog");
            log.insert(log_schema().message_key(), msg);

            log
        };

        assert_event_data_eq!(log, want);
    }

    #[test]
    fn rsyslog_omfwd_tcp_default() {
        let msg = "start";
        let raw = format!(
            r#"<190>Feb 13 21:31:56 74794bfb6795 liblogging-stdlog:  [origin software="rsyslogd" swVersion="8.24.0" x-pid="8979" x-info="http://www.rsyslog.com"] {}"#,
            msg
        );
        let log = event_from_bytes("host", raw.into()).unwrap();

        let value = log.get(log_schema().timestamp_key()).unwrap();
        let year = value.as_timestamp().unwrap().naive_local().year();
        let date: DateTime<Utc> = chrono::Local
            .with_ymd_and_hms(year, 2, 13, 21, 31, 56)
            .unwrap()
            .into();

        let want = {
            let mut log = LogRecord::from(value!({
                "host": "74794bfb6795",
                "hostname": "74794bfb6795",
                "severity": "info",
                "facility": "local7",
                "appname": "liblogging-stdlog",
                "origin": {
                    "software": "rsyslogd",
                    "swVersion": "8.24.0",
                    "x-pid": "8979",
                    "x-info": "http://www.rsyslog.com",
                }
            }));

            log.insert(log_schema().timestamp_key(), date);
            log.insert(log_schema().message_key(), msg);
            log.insert(log_schema().source_type_key(), "syslog");

            log
        };

        assert_event_data_eq!(log, want);
    }

    #[test]
    fn rsyslog_omfwd_tcp_forward_format() {
        let msg = "start";
        let raw = format!(
            r#"<190>2019-02-13T21:53:30.605850+00:00 74794bfb6795 liblogging-stdlog:  [origin software="rsyslogd" swVersion="8.24.0" x-pid="9043" x-info="http://www.rsyslog.com"] {}"#,
            msg
        );
        let event = event_from_bytes("host", raw.into()).unwrap();

        let dt = NaiveDate::from_ymd_opt(2019, 2, 13)
            .unwrap()
            .and_hms_micro_opt(21, 53, 30, 605_850)
            .unwrap();

        let want = {
            let mut log = LogRecord::from(value!({
                "host": "74794bfb6795",
                "hostname": "74794bfb6795",
                "severity": "info",
                "facility": "local7",
                "appname": "liblogging-stdlog",
                "origin": {
                    "software": "rsyslogd",
                    "swVersion": "8.24.0",
                    "x-pid": "9043",
                    "x-info": "http://www.rsyslog.com",
                }
            }));

            log.insert(log_schema().timestamp_key(), Utc.from_utc_datetime(&dt));
            log.insert(log_schema().message_key(), msg);
            log.insert(log_schema().source_type_key(), "syslog");

            log
        };

        assert_event_data_eq!(event, want);
    }
}
