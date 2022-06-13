use chrono::Utc;
use event::Event;
use framework::config::{
    DataType, GenerateConfig, Output, SourceConfig, SourceContext, SourceDescription,
};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::Source;
use futures::StreamExt;
use futures_util::stream;
use log_schema::log_schema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InternalLogsConfig {
    host_key: Option<String>,
    pid_key: Option<String>,
}

impl GenerateConfig for InternalLogsConfig {
    fn generate_config() -> String {
        r#"# No need to config anything
{}"#
        .into()
    }
}

inventory::submit! {
    SourceDescription::new::<InternalLogsConfig>("internal_logs")
}

#[async_trait::async_trait]
#[typetag::serde(name = "internal_logs")]
impl SourceConfig for InternalLogsConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let host_key = self
            .host_key
            .as_deref()
            .unwrap_or_else(|| log_schema().host_key())
            .to_owned();
        let pid_key = self.pid_key.as_deref().unwrap_or("pid").to_owned();

        Ok(Box::pin(run(host_key, pid_key, cx.output, cx.shutdown)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn source_type(&self) -> &'static str {
        "internal_logs"
    }
}

async fn run(
    host_key: String,
    pid_key: String,
    mut output: Pipeline,
    shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let subscription = framework::trace::subscribe();
    let hostname = crate::hostname();
    let pid = std::process::id();

    // chain the logs emitted before the source started first
    let mut rx = stream::iter(subscription.buffer)
        .map(Ok)
        .chain(tokio_stream::wrappers::BroadcastStream::new(
            subscription.receiver,
        ))
        .take_until(shutdown);

    // Note: This loop, or anything called within it, MUST NOT generate any
    // logs that don't break the loop, as that could cause an infinite loop
    // since it receives all such logs
    while let Some(res) = rx.next().await {
        if let Ok(mut log) = res {
            if let Ok(hostname) = &hostname {
                log.insert_field(host_key.clone(), hostname.to_owned());
            }

            log.insert_field(pid_key.clone(), pid);
            log.insert_field(log_schema().source_type_key(), "internal_log");
            log.insert_field(log_schema().timestamp_key(), Utc::now());
            if let Err(err) = output.send(Event::from(log)).await {
                error!(
                    message = "Error sending log",
                    %err
                );

                return Err(());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::log::Value;
    use futures::Stream;
    use std::time::Duration;
    use testify::collect_ready;
    use tokio::time::sleep;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<InternalLogsConfig>();
    }

    #[tokio::test]
    async fn receive_logs() {
        let test_id: u8 = rand::random();
        let start = chrono::Utc::now();
        framework::trace::init(false, false, "debug");
        framework::trace::reset_early_buffer();
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
        let test_id = Value::from(test_id.to_string());
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

    async fn start_source() -> impl Stream<Item = Event> {
        let (tx, rx) = Pipeline::new_test();
        let source = InternalLogsConfig::default()
            .build(SourceContext::new_test(tx))
            .await
            .unwrap();

        tokio::spawn(source);
        sleep(Duration::from_millis(10)).await;
        framework::trace::stop_buffering();
        rx
    }
}
