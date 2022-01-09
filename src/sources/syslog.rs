use crate::codecs;
use crate::codecs::framing::bytes::BytesDecoder;
use crate::codecs::framing::octet_counting::OctetCountingDecoder;
use crate::codecs::{Decoder, SyslogDeserializer};
use crate::config::SourceDescription;
use crate::config::{DataType, GenerateConfig, Resource, SourceConfig, SourceContext};
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;
use crate::sources::utils::{build_unix_stream_source, SocketListenAddr, TcpNullAcker, TcpSource};
use crate::sources::Source;
use crate::tcp::TcpKeepaliveConfig;
use crate::tls::{MaybeTlsSettings, TlsConfig};
use crate::udp;
use bytes::Bytes;
use chrono::Utc;
use event::Event;
use futures_util::{FutureExt, SinkExt, StreamExt};
use humanize::{deserialize_bytes_option, serialize_bytes_option};
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio_util::udp::UdpFramed;

// The default max length of the input buffer
const fn default_max_length() -> usize {
    128 * 1024
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Mode {
    Tcp {
        address: SocketListenAddr,
        keepalive: Option<TcpKeepaliveConfig>,
        tls: Option<TlsConfig>,
        #[serde(
            default,
            deserialize_with = "deserialize_bytes_option",
            serialize_with = "serialize_bytes_option"
        )]
        receive_buffer_bytes: Option<usize>,
        connection_limit: Option<u32>,
    },
    Udp {
        address: SocketAddr,
        #[serde(
            default,
            deserialize_with = "deserialize_bytes_option",
            serialize_with = "serialize_bytes_option"
        )]
        receive_buffer_bytes: Option<usize>,
    },
    #[cfg(unix)]
    Unix { path: PathBuf },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SyslogConfig {
    #[serde(flatten)]
    mode: Mode,
    #[serde(default = "default_max_length")]
    max_length: usize,
    // The host key of the log. This differs from `hostname`
    host_key: Option<String>,
}

impl GenerateConfig for SyslogConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::to_value(Self {
            mode: Mode::Tcp {
                address: SocketListenAddr::SocketAddr("0.0.0.0:514".parse().unwrap()),
                keepalive: None,
                tls: None,
                receive_buffer_bytes: None,
                connection_limit: None,
            },
            max_length: default_max_length(),
            host_key: None,
        })
        .unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<SyslogConfig>("syslog")
}

#[async_trait::async_trait]
#[typetag::serde(name = "syslog")]
impl SourceConfig for SyslogConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
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
                    ctx,
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
                ctx.shutdown,
                ctx.output,
            )),

            #[cfg(unix)]
            Mode::Unix { path } => {
                let decoder = Decoder::new(
                    Box::new(OctetCountingDecoder::new_with_max_length(self.max_length)),
                    Box::new(SyslogDeserializer),
                );

                Ok(build_unix_stream_source(
                    path,
                    decoder,
                    move |events, host, byte_size| {
                        handle_events(events, &host_key, host, byte_size)
                    },
                    ctx.shutdown,
                    ctx.output,
                ))
            }
        }
    }

    fn output_type(&self) -> DataType {
        DataType::Log
    }

    fn source_type(&self) -> &'static str {
        "syslog"
    }

    fn resources(&self) -> Vec<Resource> {
        match self.mode.clone() {
            Mode::Tcp { address, .. } => vec![address.into()],
            Mode::Udp { address, .. } => vec![Resource::udp(address)],
            #[cfg(unix)]
            Mode::Unix { path, .. } => vec![],
        }
    }
}

#[derive(Debug, Clone)]
struct SyslogTcpSource {
    max_length: usize,
    host_key: String,
}

impl TcpSource for SyslogTcpSource {
    type Error = codecs::decoding::Error;
    type Item = SmallVec<[Event; 1]>;
    type Decoder = codecs::Decoder;
    type Acker = TcpNullAcker;

    fn decoder(&self) -> Self::Decoder {
        codecs::Decoder::new(
            Box::new(OctetCountingDecoder::new_with_max_length(self.max_length)),
            Box::new(SyslogDeserializer),
        )
    }

    fn build_acker(&self, item: &[Self::Item]) -> Self::Acker {
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
) -> super::Source {
    Box::pin(async move {
        let socket = UdpSocket::bind(&addr)
            .await
            .expect("Failed to bind to UDP listener socket");

        if let Some(receive_buffer_bytes) = receive_buffer_bytes {
            if let Err(err) = udp::set_receive_buffer_size(&socket, receive_buffer_bytes) {
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
            codecs::Decoder::new(Box::new(BytesDecoder::new()), Box::new(SyslogDeserializer)),
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
                            internal_log_rate_secs = 10
                        );

                        None
                    }
                }
            }
        })
        .boxed();

        Ok(())
        /*
        match output.send_all(&mut stream).await {
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
        }*/
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
        let log = event.as_mut_log();

        log.insert_field(log_schema().source_type_key(), Bytes::from("syslog"));

        if let Some(default_host) = &default_host {
            log.insert_field("source_ip", default_host.clone());
        }

        let parsed_hostname = log.get_field("hostname").map(|h| h.as_bytes());
        if let Some(parsed_host) = parsed_hostname.or(default_host) {
            log.insert_field(host_key, parsed_host);
        }

        let timestamp = log
            .get_field("timestamp")
            .and_then(|ts| ts.as_timestamp().cloned())
            .unwrap_or_else(Utc::now);
        log.insert_field(log_schema().timestamp_key(), timestamp);
    }
}
