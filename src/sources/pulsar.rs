use std::path::PathBuf;

use chrono::TimeZone;
use codecs::decoding::{DecodeError, DeserializerConfig, FramingConfig, StreamDecodingError};
use codecs::{Decoder, DecodingConfig};
use configurable::{Configurable, configurable_component};
use event::Events;
use finalize::{AddBatchNotifier, BatchNotifier, BatchStatus, OrderedFinalizer};
use framework::config::{Output, SecretString, SourceConfig, SourceContext};
use framework::{Pipeline, ShutdownSignal, Source};
use futures::StreamExt;
use pulsar::authentication::oauth2::{OAuth2Authentication, OAuth2Params};
use pulsar::consumer::{DeadLetterPolicy, Message};
use pulsar::proto::MessageIdData;
use pulsar::{Authentication, Consumer, ConsumerOptions, Pulsar, SubType, TokioExecutor};
use serde::{Deserialize, Serialize};
use tokio_util::codec::FramedRead;

use super::default_framing_message_based;

/// OAuth2-specific authentication configuration
#[derive(Configurable, Debug, Deserialize, Serialize)]
struct OAuth2Config {
    /// The issuer URL
    issuer_url: String,

    /// The credentials URL
    ///
    /// A data URL is also supported
    credentials_url: String,

    /// The OAuth2 audience
    audience: Option<String>,

    /// The OAuth2 scope
    scope: Option<String>,
}

/// Authentication configuration
#[derive(Configurable, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, untagged)]
enum AuthConfig {
    /// Basic authentication.
    Basic {
        /// Basic authentication name/username
        ///
        /// This can be used either for basic authentication(username/password) or
        /// JWT authentication. When used for JWT, the value should be `token`.
        name: String,

        /// Basic authentication password/token
        ///
        /// This can be used either for basic authentication (username/password) or
        /// JWT authentication. When used for JWT, the value should be the signed
        /// JWT, in the compact representation.
        token: SecretString,
    },

    /// OAuth authentication
    OAuth { oauth2: OAuth2Config },
}

/// Dead Letter Queue Policy configuration
#[derive(Configurable, Debug, Deserialize, Serialize)]
struct DeadLetterQueuePolicy {
    /// Maximum number of times that a message will be redelivered before being
    /// sent to the dead letter queue.
    max_redeliver: usize,

    /// Name of the dead letter topic where the failing messages will be sent
    dead_letter_topic: String,
}

const fn default_true() -> bool {
    true
}

#[derive(Configurable, Debug, Deserialize, Serialize)]
struct TlsConfig {
    /// File path containing a list of PEM encoded certificates
    ca: PathBuf,

    /// Enables certificate verification
    ///
    /// Do not set theis to `false` unless you understand the risks of
    /// not verifying the validity of certificates.
    #[serde(default = "default_true")]
    verify_certificate: bool,

    /// Whether hostname verification is enabled when verify_certificate is false
    ///
    /// Set to true if not specified
    #[serde(default = "default_true")]
    verify_hostname: bool,
}

/// Collect observability events from Apache Pulsar topics
#[configurable_component(source, name = "pulsar")]
struct Config {
    /// The endpoint to which the Pulsar client should connect to.
    #[configurable(example = "pulsar://127.0.0.1:6650")]
    endpoint: String,

    /// The Pulsar topic names to read events from
    topics: Vec<String>,

    /// The Pulsar consumer name.
    consumer_name: Option<String>,

    /// The Pulsar subscription name.
    subscription_name: Option<String>,

    /// The consumer's priority level
    ///
    /// The broker follows descending priorities. For example, 0=max-priority, 1, 2...
    ///
    /// In shared subscription type, the broker first dispatches messages to the max
    /// priority level consumers if they have permits. Otherwise, the broker considers
    /// next priority level consumers.
    priority_level: Option<i32>,

    /// Max count of messages in a batch
    batch_size: Option<u32>,

    auth: Option<AuthConfig>,

    dead_letter_queue_policy: Option<DeadLetterQueuePolicy>,

    #[serde(default = "default_framing_message_based")]
    framing: FramingConfig,

    decoding: DeserializerConfig,

    tls: Option<TlsConfig>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "pulsar")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let consumer = self.build_consumer().await?;
        let decoder = DecodingConfig::new(self.framing.clone(), self.decoding.clone()).build()?;
        let acknowledgements = cx.acknowledgements;

        Ok(Box::pin(run(
            consumer,
            decoder,
            acknowledgements,
            cx.output,
            cx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn can_acknowledge(&self) -> bool {
        true
    }
}

impl Config {
    async fn build_consumer(&self) -> crate::Result<Consumer<Vec<u8>, TokioExecutor>> {
        let mut builder = Pulsar::builder(&self.endpoint, TokioExecutor);
        if let Some(auth) = self.auth.as_ref() {
            builder = match auth {
                AuthConfig::Basic { name, token } => builder.with_auth(Authentication {
                    name: name.clone(),
                    data: token.to_string().into_bytes(),
                }),
                AuthConfig::OAuth { oauth2 } => builder.with_auth_provider(
                    OAuth2Authentication::client_credentials(OAuth2Params {
                        issuer_url: oauth2.issuer_url.clone(),
                        credentials_url: oauth2.credentials_url.clone(),
                        audience: oauth2.audience.clone(),
                        scope: oauth2.scope.clone(),
                    }),
                ),
            };
        }

        if let Some(tls) = &self.tls {
            builder = builder
                .with_certificate_chain_file(tls.ca.clone())?
                .with_allow_insecure_connection(tls.verify_certificate)
                .with_tls_hostname_verification_enabled(tls.verify_hostname);
        }

        let pulsar = builder.build().await?;

        let mut builder = pulsar
            .consumer()
            .with_topics(&self.topics)
            .with_subscription_type(SubType::Shared)
            .with_options(ConsumerOptions {
                priority_level: self.priority_level,
                ..Default::default()
            });

        if let Some(dead_letter_queue_policy) = &self.dead_letter_queue_policy {
            builder = builder.with_dead_letter_policy(DeadLetterPolicy {
                max_redeliver_count: dead_letter_queue_policy.max_redeliver,
                dead_letter_topic: dead_letter_queue_policy.dead_letter_topic.clone(),
            });
        }

        if let Some(batch_size) = self.batch_size {
            builder = builder.with_batch_size(batch_size);
        }
        if let Some(name) = &self.consumer_name {
            builder = builder.with_consumer_name(name);
        }
        if let Some(subscription) = &self.subscription_name {
            builder = builder.with_subscription(subscription);
        }

        builder.build::<Vec<u8>>().await.map_err(Into::into)
    }
}

#[derive(Debug)]
struct FinalizerEntry {
    topic: String,
    message_id: MessageIdData,
}

async fn run(
    mut consumer: Consumer<Vec<u8>, TokioExecutor>,
    decoder: Decoder,
    acknowledgements: bool,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let (finalizer, mut ack_stream) =
        OrderedFinalizer::<FinalizerEntry>::maybe_new(acknowledgements, Some(shutdown.clone()));

    // TODO: add metrics

    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            entry = ack_stream.next() => {
                if let Some((status, entry)) = entry {
                    handle_ack(&mut consumer, status, entry).await;
                }
            },
            Some(maybe_msg) = consumer.next() => match maybe_msg {
                Ok(msg) => {
                    match parse_message(&msg, decoder.clone()).await {
                        Ok(mut events) => {
                            match finalizer.as_ref() {
                                Some(finalizer) => {
                                    let (batch, receiver) = BatchNotifier::new_with_receiver();
                                    events.for_each_log(|log| {
                                        log.add_batch_notifier(batch.clone());
                                    });
                                    drop(batch);

                                    if let Err(_err) = output.send(events).await {
                                        return Ok(());
                                    }

                                    finalizer.add(FinalizerEntry { topic: msg.topic, message_id: msg.message_id.id }, receiver);
                                },
                                None => if let Err(_err) = output.send(events).await {
                                    return Ok(());
                                }
                            }
                        },
                        Err(err) => {
                            error!(
                                message = "decode message failed",
                                ?err,
                                internal_log_rate_secs = 30
                            );
                        }
                    }
                },
                Err(err) => {
                    error!(
                        message = "failed to read message",
                        %err,
                        internal_log_rate_secs = 30
                    );
                }
            }
        }
    }

    Ok(())
}

async fn handle_ack(
    consumer: &mut Consumer<Vec<u8>, TokioExecutor>,
    status: BatchStatus,
    entry: FinalizerEntry,
) {
    match status {
        BatchStatus::Delivered => {
            if let Err(err) = consumer
                .ack_with_id(entry.topic.as_str(), entry.message_id.clone())
                .await
            {
                error!(
                    message = "Failed to acknowledge message",
                    ?err,
                    topic = entry.topic.as_str(),
                    message = ?entry.message_id,
                    internal_log_rate_secs = 30,
                )
            }
        }
        BatchStatus::Errored | BatchStatus::Failed => {
            if let Err(err) = consumer
                .nack_with_id(entry.topic.as_str(), entry.message_id.clone())
                .await
            {
                error!(
                    message = "Failed to negatively acknowledge message",
                    ?err,
                    topic = entry.topic.as_str(),
                    message = ?entry.message_id,
                    internal_log_rate_secs = 30
                );
            }
        }
    }
}

async fn parse_message(msg: &Message<Vec<u8>>, decoder: Decoder) -> Result<Events, DecodeError> {
    let publish_time = i64::try_from(msg.payload.metadata.publish_time)
        .ok()
        .and_then(|millis| chrono::Utc.timestamp_millis_opt(millis).latest());
    let topic = &msg.topic;
    let producer = &msg.payload.metadata.producer_name;
    let mut stream = FramedRead::new(msg.payload.data.as_slice(), decoder);

    let mut batch = Events::Logs(vec![]);
    while let Some(result) = stream.next().await {
        match result {
            Ok((mut events, _size)) => {
                events.for_each_log(|log| {
                    if let Some(publish_time) = publish_time {
                        log.insert("publish_time", publish_time);
                    }

                    log.insert("topic", topic.clone());
                    log.insert("producer", producer.clone());
                });

                if batch.is_empty() {
                    batch = events;
                } else {
                    batch.merge(events);
                }
            }
            Err(err) => {
                if !err.can_continue() {
                    return Err(err);
                }
            }
        }
    }

    Ok(batch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate() {
        crate::testing::generate_config::<Config>();
    }
}

#[cfg(all(test, feature = "pulsar_integration-tests"))]
mod integration_tests {
    use std::time::Duration;

    use testify::container::Container;
    use testify::next_addr;
    use testify::random::random_string;

    use super::*;
    use crate::testing::trace_init;

    async fn run(acknowledgements: bool) {
        trace_init();

        let service_addr = next_addr();
        let http_addr = next_addr();

        // https://pulsar.apache.org/docs/next/getting-started-docker/
        Container::new("apachepulsar/pulsar", "4.0.6")
            .with_tcp(6650, service_addr.port())
            .with_tcp(8080, http_addr.port())
            .args(["bin/pulsar", "standalone"])
            .run(async move {
                // wait for pulsar ready
                let topic = format!("test-{}", random_string(10));
                let endpoint = format!("pulsar://127.0.0.1:{}", service_addr.port());
                let pulsar = Pulsar::<TokioExecutor>::builder(&endpoint, TokioExecutor)
                    .build()
                    .await
                    .unwrap();

                let mut producer = loop {
                    match pulsar
                        .producer()
                        .with_name("test")
                        .with_topic(topic.clone())
                        .build()
                        .await
                    {
                        Ok(producer) => break producer,
                        Err(err) => {
                            match err {
                                pulsar::Error::ServiceDiscovery(_) => {
                                    // ok but not ready
                                }
                                _ => panic!("unexpected error: {}", err),
                            }
                        }
                    }

                    tokio::time::sleep(Duration::from_millis(500)).await;
                };

                let msg = "test message";

                let config = Config {
                    endpoint,
                    topics: vec![topic],
                    consumer_name: None,
                    subscription_name: None,
                    priority_level: None,
                    batch_size: None,
                    auth: None,
                    dead_letter_queue_policy: None,
                    framing: default_framing_message_based(),
                    decoding: DeserializerConfig::Bytes,
                    tls: None,
                };
                let (tx, mut rx) = Pipeline::new_test();
                let mut cx = SourceContext::new_test(tx);
                cx.acknowledgements = acknowledgements;
                let task = config.build(cx).await.unwrap();
                tokio::spawn(task);

                tokio::task::yield_now().await;

                let _receipt = producer
                    .send_non_blocking(msg.as_bytes())
                    .await
                    .unwrap()
                    .await
                    .unwrap();

                let events = rx.recv().await.unwrap();

                assert_eq!(
                    events.into_logs().unwrap().first().unwrap()["message"].to_string_lossy(),
                    msg
                );
            })
            .await;
    }

    #[tokio::test]
    async fn consume_with_acknowledgements() {
        // This annoying me too, because rustls does not allow users enable `ring` and `aws_lc_rs` at the same time. So adding pulsar will panic once TLS is used.
        run(true).await;
    }

    #[tokio::test]
    async fn consume_without_acknowledgements() {
        run(false).await;
    }
}
