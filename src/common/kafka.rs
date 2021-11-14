use std::path::{Path, PathBuf};

use rdkafka::{ClientConfig, ClientContext, Statistics};
use rdkafka::consumer::ConsumerContext;
use snafu::Snafu;
use serde::{Deserialize, Serialize};
use metrics::{gauge, counter};
use internal::{InternalEvent, update_counter};

use crate::Error;
use crate::tls::TLSConfig;


#[derive(Debug, Snafu)]
enum KafkaError {
    #[snafu(display("invalid path: {:?}", path))]
    InvalidPath { path: PathBuf },
}

/// Used to determine the options to set in configs, since both Kafka consumers and producers have
/// unique options, they use the same struct, and the error if given the wrong options.
#[derive(Debug, PartialOrd, PartialEq)]
pub enum KafkaRole {
    Consumer,
    Producer,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KafkaCompression {
    None,
    Gzip,
    Snappy,
    Lz4,
    Zstd,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct KafkaSaslConfig {
    enabled: Option<bool>,
    username: Option<String>,
    password: Option<String>,
    mechanism: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KafkaTLSConfig {
    pub enabled: Option<bool>,
    pub options: TLSConfig,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct KafkaAuthConfig {
    sasl: Option<KafkaSaslConfig>,
    tls: Option<KafkaTLSConfig>,
}

impl KafkaAuthConfig {
    pub fn apply(&self, client: &mut ClientConfig) -> Result<(), Error> {
        let sasl_enabled = self.sasl
            .as_ref()
            .and_then(|s| s.enabled)
            .unwrap_or(false);
        let tls_enabled = self.tls
            .as_ref()
            .and_then(|s| s.enabled)
            .unwrap_or(false);

        let protocol = match (sasl_enabled, tls_enabled) {
            (false, false) => "plaintext",
            (false, true) => "ssl",
            (true, false) => "sasl_plaintext",
            (true, true) => "sasl_ssl",
        };
        client.set("security.protocol", protocol);

        if sasl_enabled {
            let sasl = self.sasl.as_ref().unwrap();
            if let Some(username) = &sasl.username {
                client.set("sasl.username", username);
            }
            if let Some(password) = &sasl.password {
                client.set("sasl.password", password);
            }
            if let Some(mechanism) = &sasl.mechanism {
                client.set("sasl.mechanism", mechanism);
            }
        }

        if tls_enabled {
            let tls = self.tls.as_ref().unwrap();
            if let Some(path) = &tls.options.ca_file {
                client.set("ssl.ca.location", pathbuf_to_string(path)?);
            }
            if let Some(path) = &tls.options.crt_file {
                client.set("ssl.certificate.location", pathbuf_to_string(path)?);
            }
            if let Some(path) = &tls.options.key_file {
                client.set("ssl.key.location", pathbuf_to_string(path)?);
            }
            if let Some(pass) = &tls.options.key_pass {
                client.set("ssl.key.password", pass);
            }
        }

        Ok(())
    }
}

pub fn pathbuf_to_string(path: &Path) -> Result<&str, Error> {
    path.to_str()
        .ok_or_else(|| KafkaError::InvalidPath { path: path.into() }.into())
}

pub struct KafkaStatisticsContext;

impl ClientContext for KafkaStatisticsContext {
    fn stats(&self, statistics: Statistics) {
        emit!(&KafkaStatisticsReceived { statistics: &statistics })
    }
}

impl ConsumerContext for KafkaStatisticsContext {}


pub struct KafkaStatisticsReceived<'a> {
    statistics: &'a rdkafka::Statistics,
}

impl InternalEvent for KafkaStatisticsReceived<'_> {
    fn emit_metrics(&self) {
        gauge!("kafka_queue_messages", self.statistics.msg_cnt as f64);
        gauge!("kafka_queue_messages_bytes", self.statistics.msg_size as f64);
        update_counter!("kafka_requests_total", self.statistics.tx as u64);
        update_counter!("kafka_requests_bytes_total", self.statistics.tx_bytes as u64);
        update_counter!("kafka_responses_total", self.statistics.rx as u64);
        update_counter!("kafka_responses_bytes_total", self.statistics.rx_bytes as u64);
        update_counter!("kafka_produced_messages_total", self.statistics.txmsgs as u64);
        update_counter!("kafka_produced_messages_bytes_total", self.statistics.txmsg_bytes as u64);
        update_counter!("kafka_consumed_messages_total", self.statistics.rxmsgs as u64);
        update_counter!("kafka_consumed_messages_bytes_total", self.statistics.rxmsg_bytes as u64);
    }
}

pub struct KafkaHeaderExtractionFailed<'a> {
    pub headers_field: &'a str,
}

impl<'a> InternalEvent for KafkaHeaderExtractionFailed<'a> {
    fn emit_logs(&self) {
        warn!(
            message = "Failed to extract header. Value should be a map of String -> Bytes",
            header_field = self.headers_field
        )
    }

    fn emit_metrics(&self) {
        counter!("kafka_header_extraction_failures_total", 1);
    }
}