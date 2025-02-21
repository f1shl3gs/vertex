use std::net::SocketAddr;

use chrono::TimeZone;
use codecs::decoding::{DeserializerConfig, FramingConfig, StreamDecodingError};
use codecs::{Decoder, DecodingConfig};
use configurable::{configurable_component, Configurable};
use event::{AddBatchNotifier, BatchNotifier, BatchStatus, Events};
use framework::config::{Output, SecretString, SourceConfig, SourceContext};
use framework::source::OrderedFinalizer;
use framework::tls::TlsConfig;
use framework::{Pipeline, ShutdownSignal, Source};
use futures_util::StreamExt;
use metrics::Attributes;
use pulsar::authentication::oauth2::{OAuth2Authentication, OAuth2Params};
use pulsar::consumer::{DeadLetterPolicy, Message};
use pulsar::proto::MessageIdData;
use pulsar::{Authentication, Consumer, ConsumerOptions, Payload, Pulsar, SubType, TokioExecutor};
use serde::{Deserialize, Serialize};
use tokio_util::codec::FramedRead;

/// OAuth2-specific authentication configuration.
#[derive(Configurable, Debug, Deserialize, Serialize)]
struct OAuth2Config {
    /// The issuer URL
    issuer_url: String,

    /// The credentials URL
    ///
    /// A data URL is also supported.
    credentials_url: String,

    /// The OAuth2 audience.
    audience: Option<String>,

    /// The OAuth2 scope
    scope: Option<String>,
}

/// Authentication configuration
#[derive(Configurable, Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum AuthConfig {
    /// Basic authentication
    Basic {
        /// Basic authentication name/username
        ///
        /// This can be used either for basic authentication (username/password)
        /// or JWT authentication. When used for JWT, the value should be `token`.
        name: String,

        /// Basic authentication password/token
        ///
        /// This can be used either for basic authentication (username/password)
        /// or JWT authentication. When used for JWT, the value should be the signed
        /// JWT, in the compact representation.
        token: SecretString,
    },

    /// OAuth authentication
    OAuth { oauth2: OAuth2Config },
}

/// Dead Letter Queue policy configuration
#[derive(Configurable, Debug, Deserialize, Serialize)]
struct DeadLetterQueuePolicy {
    /// Maximum number of times that a message will be redelivered before being
    /// sent to the dead letter queue.
    max_redeliver_count: usize,

    /// Name of the dead letter topic where the failing messages will be sent
    dead_letter_topic: String,
}

#[configurable_component(source, name = "pulsar")]
struct Config {
    /// The endpoint to which the Pulsar client should connect to.
    endpoint: SocketAddr,

    tls: Option<TlsConfig>,

    /// The Pulsar topic names to read events from.
    topics: Vec<String>,

    /// The Pulsar consumer name.
    consumer_name: Option<String>,

    /// The Pulsar subscription name.
    subscription_name: Option<String>,

    /// The consumer's priority level.
    ///
    /// The broker follows descending priorities. For example, 0=max-priority, 1, 2...
    ///
    /// In Shared subscription type, the broker first dispatches messages to the max
    /// priority level consumers if they have permits. Otherwise, the broker considers
    /// next priority level consumers.
    priority_level: Option<i32>,

    /// Max count of messages in a batch
    batch_size: Option<u32>,

    auth: Option<AuthConfig>,

    dead_letter_queue_policy: Option<DeadLetterQueuePolicy>,

    framing: FramingConfig,

    decoding: DeserializerConfig,
}

#[async_trait::async_trait]
#[typetag::serde(name = "pulsar")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let decoder = DecodingConfig::new(self.framing.clone(), self.decoding.clone()).build()?;
        let consumer = self.build_consumer().await?;

        Ok(Box::pin(pulsar_source(
            consumer,
            decoder,
            cx.acknowledgements,
            cx.shutdown,
            cx.output,
            cx.key.to_string(),
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
        let url = format!("pulsar://{}", self.endpoint);
        let mut builder = Pulsar::builder(url, TokioExecutor);

        if let Some(auth) = self.auth.as_ref() {
            builder = match auth {
                AuthConfig::Basic { name, token } => builder.with_auth(Authentication {
                    name: name.clone(),
                    data: token.inner().as_bytes().to_vec(),
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

        let pulsar = builder.build().await?;

        let mut builder = pulsar
            .consumer()
            .with_topics(&self.topics)
            .with_subscription_type(SubType::Shared)
            .with_options(ConsumerOptions {
                priority_level: self.priority_level,
                ..Default::default()
            });

        if let Some(dead_letter_queue_policy) = self.dead_letter_queue_policy.as_ref() {
            builder = builder.with_dead_letter_policy(DeadLetterPolicy {
                max_redeliver_count: dead_letter_queue_policy.max_redeliver_count,
                dead_letter_topic: dead_letter_queue_policy.dead_letter_topic.clone(),
            });
        }

        if let Some(batch_size) = self.batch_size.as_ref() {
            builder = builder.with_batch_size(*batch_size);
        }

        if let Some(name) = self.consumer_name.as_ref() {
            builder = builder.with_consumer_name(name);
        }

        if let Some(name) = self.subscription_name.as_ref() {
            builder = builder.with_subscription(name);
        }

        builder.build().await.map_err(Into::into)
    }
}

#[derive(Debug)]
struct FinalizerEntry {
    topic: String,
    message_id: MessageIdData,
}

async fn pulsar_source(
    mut consumer: Consumer<Vec<u8>, TokioExecutor>,
    decoder: Decoder,
    acknowledgements: bool,
    mut shutdown: ShutdownSignal,
    mut output: Pipeline,
    source_name: String,
) -> Result<(), ()> {
    let (finalizer, mut ack_stream) =
        OrderedFinalizer::<FinalizerEntry>::maybe_new(acknowledgements, shutdown.clone());

    let attrs = Attributes::from([("source", source_name.into())]);
    let bytes_received =
        metrics::register_counter("pulsar_bytes_received", "").recorder(attrs.clone());
    let events_received =
        metrics::register_counter("pulsar_events_received", "").recorder(attrs.clone());
    let error_events = metrics::register_counter("pulsar_error_events", "").recorder(attrs);

    loop {
        let maybe_msg = tokio::select! {
            _ = &mut shutdown => break,
            entry = ack_stream.next() => {
                if let Some((status, entry)) = entry {
                    handle_ack(&mut consumer, status, entry).await;
                    events_received.inc(1)
                }

                continue;
            },
            Some(maybe_msg) = consumer.next() => maybe_msg
        };

        match maybe_msg {
            Ok(Message {
                topic,
                payload,
                message_id,
                ..
            }) => {
                bytes_received.inc(payload.data.len() as u64);

                let mut events = match parse_message(&topic, payload, decoder.clone()).await {
                    Ok(events) => events,
                    Err(err) => {
                        warn!(message = "parse message failed", ?err);

                        continue;
                    }
                };

                match finalizer.as_ref() {
                    Some(finalizer) => {
                        let (batch, receiver) = BatchNotifier::new_with_receiver();
                        events.add_batch_notifier(batch);

                        match output.send(events).await {
                            Ok(_) => {
                                finalizer.add(
                                    FinalizerEntry {
                                        topic,
                                        message_id: message_id.id,
                                    },
                                    receiver,
                                );
                            }
                            Err(_err) => break,
                        }
                    }
                    None => match output.send(events).await {
                        Ok(_) => {
                            if let Err(err) =
                                consumer.ack_with_id(topic.as_str(), message_id.id).await
                            {
                                error!(
                                    message = "Failed to acknowledge message",
                                    ?err,
                                    internal_log_rate_secs = 30
                                );
                            }
                        }
                        Err(_err) => return Ok(()),
                    },
                }
            }
            Err(err) => {
                error!(
                    message = "Failed to read message",
                    ?err,
                    internal_log_rate_secs = 30
                );

                error_events.inc(1);
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
                .ack_with_id(entry.topic.as_str(), entry.message_id)
                .await
            {
                error!(
                    message = "Failed to acknowledge message",
                    ?err,
                    internal_log_rate_secs = 30
                );
            }
        }
        BatchStatus::Errored | BatchStatus::Failed => {
            if let Err(err) = consumer
                .nack_with_id(entry.topic.as_str(), entry.message_id)
                .await
            {
                error!(
                    message = "Failed to negatively acknowledge message",
                    ?err,
                    internal_log_rate_secs = 30
                )
            }
        }
    }
}

async fn parse_message(topic: &str, payload: Payload, decoder: Decoder) -> crate::Result<Events> {
    let publish_time = i64::try_from(payload.metadata.publish_time)
        .ok()
        .and_then(|millis| chrono::Utc.timestamp_millis_opt(millis).latest());
    let producer_name = payload.metadata.producer_name.clone();

    let mut stream = FramedRead::new(payload.data.as_ref(), decoder);

    let mut batch = Events::Logs(vec![]);
    while let Some(result) = stream.next().await {
        match result {
            Ok((mut events, _bytes)) => {
                events.for_each_log(|log| {
                    if let Some(publish_time) = publish_time {
                        log.insert("publish_time", publish_time);
                    }

                    log.insert("topic", topic);
                    log.insert("producer_name", producer_name.clone());
                });

                if batch.is_empty() {
                    batch = events;
                } else {
                    batch.merge(events);
                }
            }
            Err(err) => {
                if !err.can_continue() {
                    break;
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
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}

#[cfg(all(test, feature = "pulsar-integration-tests"))]
mod integration_tests {
    use std::time::Duration;

    use testify::container::Container;
    use testify::random::random_string;
    use testify::wait::wait_for_tcp;
    use testify::{collect_ready, next_addr};

    use super::*;
    use crate::testing::trace_init;

    async fn send_message(service: SocketAddr, topic: &str) {
        let pc = Pulsar::builder(format!("pulsar://{}", service), TokioExecutor)
            .build()
            .await
            .unwrap();

        let mut producer = pc.producer().with_topic(topic).build().await.unwrap();

        for i in 0..10 {
            producer
                .send_non_blocking(format!("message {i}"))
                .await
                .unwrap()
                .await
                .unwrap();
        }
    }

    async fn send_and_receive(tag: &str, acknowledgement: bool) {
        trace_init();

        let broker_addr = next_addr();
        let http_addr = next_addr();

        let topic = format!("test-{}", random_string(10));
        let config = Config {
            endpoint: broker_addr,
            tls: None,
            topics: vec![topic.clone()],
            consumer_name: None,
            subscription_name: None,
            priority_level: None,
            batch_size: None,
            auth: None,
            dead_letter_queue_policy: None,
            framing: FramingConfig::Bytes,
            decoding: DeserializerConfig::Bytes,
        };

        let (pipeline, rx) = Pipeline::new_test();
        let mut cx = SourceContext::new_test(pipeline);
        cx.acknowledgements = acknowledgement;

        let received = Container::new("apachepulsar/pulsar", tag)
            .with_tcp(6650, broker_addr.port())
            .with_tcp(8080, http_addr.port())
            .args(["bin/pulsar", "standalone"])
            .tail_logs(true, true)
            .run(async move {
                tokio::time::sleep(Duration::from_secs(20)).await;

                wait_for_tcp(broker_addr).await;

                let src = config.build(cx).await.unwrap();

                tokio::spawn(src);

                send_message(broker_addr, &topic).await;

                tokio::time::sleep(Duration::from_secs(10)).await;

                collect_ready(rx).await
            })
            .await;

        let total = received.iter().map(|events| events.len()).sum::<usize>();

        assert_eq!(total, 10);
    }

    #[tokio::test]
    async fn batch() {
        send_and_receive("4.0.2", true).await;
    }
}
