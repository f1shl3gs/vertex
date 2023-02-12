mod util;

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use codecs::encoding::{FramingConfig, SerializerConfig};
use codecs::EncodingConfigWithFraming;
use framework::sink::util::tcp::TcpSinkConfig;
use framework::testing::CountReceiver;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use testify::next_addr;
use testify::random::{random_maps, random_string};
use testify::send_lines;
use testify::wait::wait_for_tcp;
use vertex::sinks::socket;
use vertex::sinks::socket::Config;
use vertex::sources::syslog::{default_max_length, Mode, SyslogConfig};

use crate::util::trace_init;
use util::start_topology;

#[allow(clippy::derive_partial_eq_without_eq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Deserialize, PartialEq, Debug)]
pub enum Severity {
    #[serde(rename(deserialize = "emergency"))]
    LOG_EMERG,
    #[serde(rename(deserialize = "alert"))]
    LOG_ALERT,
    #[serde(rename(deserialize = "critical"))]
    LOG_CRIT,
    #[serde(rename(deserialize = "error"))]
    LOG_ERR,
    #[serde(rename(deserialize = "warn"))]
    LOG_WARNING,
    #[serde(rename(deserialize = "notice"))]
    LOG_NOTICE,
    #[serde(rename(deserialize = "info"))]
    LOG_INFO,
    #[serde(rename(deserialize = "debug"))]
    LOG_DEBUG,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Copy, Clone, PartialEq, Deserialize, Debug)]
pub enum Facility {
    #[serde(rename(deserialize = "kernel"))]
    LOG_KERN = 0 << 3,
    #[serde(rename(deserialize = "user"))]
    LOG_USER = 1 << 3,
    #[serde(rename(deserialize = "mail"))]
    LOG_MAIL = 2 << 3,
    #[serde(rename(deserialize = "daemon"))]
    LOG_DAEMON = 3 << 3,
    #[serde(rename(deserialize = "auth"))]
    LOG_AUTH = 4 << 3,
    #[serde(rename(deserialize = "syslog"))]
    LOG_SYSLOG = 5 << 3,
}

type StructuredData = HashMap<String, HashMap<String, String>>;

#[derive(Deserialize, PartialEq, Clone, Debug)]
struct SyslogMessageRfc5424 {
    msgid: String,
    severity: Severity,
    facility: Facility,
    version: u8,
    timestamp: String,
    host: String,
    source_type: String,
    appname: String,
    procid: usize,
    message: String,
    #[serde(flatten)]
    structured_data: StructuredData,
}

impl SyslogMessageRfc5424 {
    fn random(
        id: usize,
        msg_len: usize,
        field_len: usize,
        max_map_size: usize,
        max_children: usize,
    ) -> Self {
        let msg = random_string(msg_len);
        let structured_data = random_structured_data(max_map_size, max_children, field_len);

        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        //"secfrac" can contain up to 6 digits, but TCP sinks uses `AutoSi`

        Self {
            msgid: format!("test{}", id),
            severity: Severity::LOG_INFO,
            facility: Facility::LOG_USER,
            version: 1,
            timestamp,
            host: "localhost.localdomain".to_owned(),
            source_type: "syslog".to_owned(),
            appname: "harry".to_owned(),
            procid: thread_rng().gen_range(0..32768),
            structured_data,
            message: msg,
        }
    }
}

impl fmt::Display for SyslogMessageRfc5424 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<{}>{} {} {} {} {} {} {} {}",
            encode_priority(self.severity, self.facility),
            self.version,
            self.timestamp,
            self.host,
            self.appname,
            self.procid,
            self.msgid,
            format_structured_data_rfc5424(&self.structured_data),
            self.message
        )
    }
}

fn format_structured_data_rfc5424(data: &StructuredData) -> String {
    if data.is_empty() {
        "-".to_string()
    } else {
        let mut res = String::new();
        for (id, params) in data {
            res = res + "[" + id;
            for (name, value) in params {
                res = res + " " + name + "=\"" + value + "\"";
            }
            res += "]";
        }

        res
    }
}

fn encode_priority(severity: Severity, facility: Facility) -> u8 {
    facility as u8 | severity as u8
}

fn random_structured_data(
    max_map_size: usize,
    max_children: usize,
    field_len: usize,
) -> StructuredData {
    let amount = thread_rng().gen_range(0..max_children);

    random_maps(max_map_size, field_len)
        .filter(|m| !m.is_empty()) //syslog_rfc5424 ignores empty maps, tested separately
        .take(amount)
        .enumerate()
        .map(|(i, map)| (format!("id{}", i), map))
        .collect()
}

fn tcp_json_sink(address: String) -> Config {
    Config::new(
        socket::Mode::Tcp(TcpSinkConfig::from_address(address)),
        EncodingConfigWithFraming::new(
            Some(FramingConfig::NewlineDelimited),
            SerializerConfig::Json,
            Default::default(),
        ),
    )
}

#[tokio::test]
async fn tcp_syslog() {
    trace_init();

    let num = 10000usize;

    let in_addr = next_addr();
    let out_addr = next_addr();

    let mut config = framework::config::Config::builder();
    config.add_source(
        "in",
        SyslogConfig {
            mode: Mode::Tcp {
                address: in_addr.into(),
                keepalive: None,
                tls: None,
                receive_buffer_bytes: None,
                connection_limit: None,
            },
            max_length: default_max_length(),
            host_key: None,
        },
    );
    config.add_sink("out", &["in"], tcp_json_sink(out_addr.to_string()));

    let output_lines = CountReceiver::receive_lines(out_addr);

    let (topology, _crash) = start_topology(config.build().unwrap(), false).await;
    // Wait for server to accept traffic
    wait_for_tcp(in_addr).await;

    let input_messages: Vec<SyslogMessageRfc5424> = (0..num)
        .map(|i| SyslogMessageRfc5424::random(i, 30, 4, 3, 3))
        .collect();

    let input_lines: Vec<String> = input_messages.iter().map(|msg| msg.to_string()).collect();

    send_lines(in_addr, input_lines).await.unwrap();

    // Shut down server
    topology.stop().await;

    let output_lines = output_lines.await;
    assert_eq!(output_lines.len(), num);

    let output_messages: Vec<SyslogMessageRfc5424> = output_lines
        .iter()
        .map(|s| {
            let mut value = serde_json::Value::from_str(s).unwrap();
            let v = value.as_object_mut().unwrap().get("fields").unwrap();
            let mut v = v.clone();
            v.as_object_mut().unwrap().remove("hostname");
            v.as_object_mut().unwrap().remove("source_ip");

            serde_json::from_value(v).unwrap()
        })
        .collect();

    assert_eq!(input_messages, output_messages);
}
