use crate::config::{DataType, GenerateConfig, Resource, SourceConfig, SourceContext};
use crate::sources::Source;
use crate::tcp::TcpKeepaliveConfig;
use crate::tls::TlsConfig;
use humanize::{deserialize_bytes_option, serialize_bytes_option};
use log_schema::log_schema;
use serde_yaml::Value;
use std::net::SocketAddr;
use std::path::PathBuf;

// The default max length of the input buffer
const fn default_max_length() -> usize {
    128 * 1024
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Mode {
    Tcp {
        address: SocketAddr,
        keepalive: Option<TcpKeepaliveConfig>,
        tls: Option<TlsConfig>,
        #[serde(
            default,
            deserialize_with = "deserialize_bytes_option",
            serialize_with = "serialize_bytes_option"
        )]
        receive_buffer_bytes: Option<usize>,
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
pub struct SyslogSourceConfig {
    #[serde(flatten)]
    mode: Mode,
    #[serde(default = "")]
    max_length: usize,
    // The host key of the log. This differs from `hostname`
    host_key: Option<String>,
}

impl GenerateConfig for SyslogSourceConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::to_value(Self {
            mode: Mode::Tcp {
                address: "0.0.0.0:514".to_string(),
                keepalive: None,
                tls: None,
                receive_buffer_bytes: None,
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
impl SourceConfig for SyslogSourceConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let host_key = self
            .host_key
            .clone()
            .unwrap_or_else(|| log_schema().host_key().to_string());
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
