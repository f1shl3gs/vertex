use chrono::Utc;
use event::Event;
use futures::{SinkExt, StreamExt};
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;

use crate::config::{DataType, SourceConfig, SourceContext, SourceDescription};
use crate::impl_generate_config_from_default;
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;
use crate::sources::Source;

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InternalLogsConfig {
    host_key: Option<String>,
    pid_key: Option<String>,
}

inventory::submit! {
    SourceDescription::new::<InternalLogsConfig>("internal_logs")
}

impl_generate_config_from_default!(InternalLogsConfig);

#[async_trait::async_trait]
#[typetag::serde(name = "internal_logs")]
impl SourceConfig for InternalLogsConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let host_key = self
            .host_key
            .as_deref()
            .unwrap_or_else(|| log_schema().host_key())
            .to_owned();
        let pid_key = self.pid_key.as_deref().unwrap_or("pid").to_owned();

        Ok(Box::pin(run(host_key, pid_key, ctx.out, ctx.shutdown)))
    }

    fn output_type(&self) -> DataType {
        DataType::Log
    }

    fn source_type(&self) -> &'static str {
        "internal_logs"
    }
}

async fn run(
    host_key: String,
    pid_key: String,
    output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut output = output.sink_map_err(|err| {
        error!(
            message = "Error sending logs",
            %err
        )
    });
    let subscription = crate::trace::subscribe();
    let mut rx = subscription.receiver;

    let hostname = crate::hostname();
    let pid = std::process::id();

    output
        .send_all(
            &mut futures::stream::iter(subscription.buffer).map(|mut log| {
                if let Ok(hostname) = &hostname {
                    log.insert_field(host_key.clone(), hostname.to_owned());
                }

                log.insert_field(pid_key.clone(), pid);
                log.try_insert_field(log_schema().source_type_key(), "internal_logs");
                log.try_insert_field(log_schema().timestamp_key(), Utc::now());

                Ok(log.into())
            }),
        )
        .await?;

    // Note: This loop, or anything called within it, MUST NOT generate any logs that don't
    // break the loop, as that could cause an infinite loop since it receives all such logs.
    loop {
        tokio::select! {
            receive = rx.recv() => {
                match receive {
                    Ok(event) => output.send(Event::from(event)).await?,
                    Err(RecvError::Lagged(_)) => (),
                    Err(RecvError::Closed) => break
                }
            }
            _ = &mut shutdown => break
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::Value;
    use futures::channel::mpsc;
    use std::time::Duration;
    use testify::collect_ready;
    use tokio::time::sleep;

    #[test]
    fn generate_config() {
        crate::config::test_generate_config::<InternalLogsConfig>();
    }

    #[tokio::test]
    async fn receive_logs() {
        let test_id: u8 = rand::random();
        let start = chrono::Utc::now();
        crate::trace::init(false, false, "debug");
        crate::trace::reset_early_buffer();
        error!(
            message = "Before source started",
            %test_id
        );

        let rx = start_source().await;
        error!(
            message = "After source started",
            %test_id
        );

        sleep(Duration::from_millis(10)).await;
        let mut events = collect_ready(rx).await;
        let mut test_id = Value::from(test_id.to_string());
        events.retain(|event| event.as_log().get_field("test_id") == Some(&test_id));

        let end = chrono::Utc::now();

        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0].as_log().fields["message"],
            "Before source started".into()
        );
        assert_eq!(
            events[1].as_log().fields["message"],
            "After source started".into()
        );

        for event in events {
            let log = event.as_log();
            let timestamp = log.fields["timestamp"]
                .as_timestamp()
                .expect("timestamp isn't a timestamp");
            assert!(*timestamp >= start);
            assert!(*timestamp <= end);
        }
    }

    async fn start_source() -> mpsc::Receiver<Event> {
        let (tx, rx) = Pipeline::new_test();
        let source = InternalLogsConfig::default()
            .build(SourceContext::new_test(tx))
            .await
            .unwrap();

        tokio::spawn(source);
        sleep(Duration::from_millis(10)).await;
        crate::trace::stop_buffering();
        rx
    }
}
