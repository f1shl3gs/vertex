use std::path::{Path, PathBuf};

use framework::config::GenerateConfig;
use framework::tls::TlsOptions;
use framework::Error;
use metrics::{Counter, Gauge};
use rdkafka::consumer::ConsumerContext;
use rdkafka::{ClientConfig, ClientContext, Statistics};
use serde::{Deserialize, Serialize};
use snafu::Snafu;

#[derive(Debug, Snafu)]
enum KafkaError {
    #[snafu(display("invalid path: {:?}", path))]
    InvalidPath { path: PathBuf },
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

impl Default for KafkaCompression {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KafkaSaslConfig {
    enabled: Option<bool>,
    username: Option<String>,
    password: Option<String>,
    mechanism: Option<String>,
}

impl GenerateConfig for KafkaSaslConfig {
    fn generate_config() -> String {
        r#"
# Enable SASL/SCRAM authentication to the remote. (Not
# supported on Windows at this time)
#
# enabled: false

# The kafka SASL/SCRAM mechanisms
#
# mechanism: SCRAM-SHA-512

# The Kafka SASL/SCRAM authentication password.
#
# password: password

# The Kafka SASL/SCRAM authentication username.
#
# username: username
"#
        .into()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KafkaTLSConfig {
    pub enabled: Option<bool>,
    pub options: TlsOptions,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct KafkaAuthConfig {
    pub sasl: Option<KafkaSaslConfig>,
    pub tls: Option<KafkaTLSConfig>,
}

impl KafkaAuthConfig {
    pub fn apply(&self, client: &mut ClientConfig) -> Result<(), Error> {
        let sasl_enabled = self.sasl.as_ref().and_then(|s| s.enabled).unwrap_or(false);
        let tls_enabled = self.tls.as_ref().and_then(|s| s.enabled).unwrap_or(false);

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

pub struct KafkaStatisticsContext {
    queue_messages: Gauge,
    queue_message_bytes: Gauge,
    requests: Counter,
    requests_bytes: Counter,
    responses: Counter,
    responses_bytes: Counter,
    produced: Counter,
    produced_bytes: Counter,
    consumed: Counter,
    consumed_bytes: Counter,
}

impl KafkaStatisticsContext {
    pub fn new() -> Self {
        Self {
            queue_messages: metrics::register_gauge(
                "kafka_queue_messages",
                "Current number of messages in producer queues.",
            ).recorder(&[]),
            queue_message_bytes: metrics::register_gauge(
                "kafka_queue_messages_bytes",
                "Current total size of messages in producer queues.",
            ).recorder(&[]),
            requests: metrics::register_counter(
                "kafka_requests_total",
                "	Total number of requests sent to Kafka brokers.",
            ).recorder(&[]),
            requests_bytes: metrics::register_counter(
                "kafka_requests_bytes_total",
                "Total number of bytes transmitted to Kafka brokers.",
            ).recorder(&[]),
            responses: metrics::register_counter(
                "kafka_responses_total",
                "Total number of responses received from Kafka brokers.",
            ).recorder(&[]),
            responses_bytes: metrics::register_counter(
                "kafka_responses_bytes_total",
                "Total number of bytes received from Kafka brokers.",
            ).recorder(&[]),
            produced: metrics::register_counter(
                "kafka_produced_messages_total",
                "Total number of messages transmitted (produced) to Kafka brokers.",
            ).recorder(&[]),
            produced_bytes: metrics::register_counter(
                "kafka_produced_messages_bytes_total",
                "Total number of message bytes (including framing, such as per-Message framing and MessageSet/batch framing) transmitted to Kafka brokers.",
            ).recorder(&[]),
            consumed: metrics::register_counter(
                "kafka_consumed_messages_total",
                "Total number of messages consumed, not including ignored messages (due to offset, etc), from Kafka brokers.",
            ).recorder(&[]),
            consumed_bytes: metrics::register_counter(
                "kafka_consumed_messages_bytes_total",
                "Total number of message bytes (including framing) received from Kafka brokers.",
            ).recorder(&[]),
        }
    }
}

impl ClientContext for KafkaStatisticsContext {
    fn stats(&self, statistics: Statistics) {
        self.queue_messages.set(statistics.msg_cnt as f64);
        self.queue_message_bytes.set(statistics.msg_size as f64);
        self.requests.set(statistics.tx as u64);
        self.requests_bytes.set(statistics.tx_bytes as u64);
        self.responses.set(statistics.rx as u64);
        self.responses_bytes.set(statistics.rx_bytes as u64);
        self.produced.set(statistics.txmsgs as u64);
        self.produced_bytes.set(statistics.txmsg_bytes as u64);
        self.consumed.set(statistics.rxmsgs as u64);
        self.consumed_bytes.set(statistics.rxmsg_bytes as u64);
    }
}

impl ConsumerContext for KafkaStatisticsContext {}
