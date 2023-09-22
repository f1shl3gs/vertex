use chrono::Utc;
use configurable::configurable_component;
use framework::config::{DataType, Output, SourceConfig, SourceContext};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::trace::TraceSubscription;
use framework::Source;
use futures::StreamExt;
use futures_util::stream;
use log_schema::log_schema;

#[configurable_component(source, name = "internal_logs")]
#[derive(Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Host key
    host_key: Option<String>,

    /// Pid key
    pid_key: Option<String>,
}

/// The internal logs source exposes all log and trace messages emitted
/// by the running Vertex instance.
#[async_trait::async_trait]
#[typetag::serde(name = "internal_logs")]
impl SourceConfig for Config {
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
}

async fn run(
    host_key: String,
    pid_key: String,
    mut output: Pipeline,
    shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut subscription = TraceSubscription::subscribe();
    let hostname = crate::hostname().expect("get hostname success");
    let pid = std::process::id();

    // chain the logs emitted before the source started first
    let mut rx = stream::iter(subscription.buffered())
        .chain(subscription.into_stream())
        .ready_chunks(128)
        .take_until(shutdown);

    // Note: This loop, or anything called within it, MUST NOT generate any
    // logs that don't break the loop, as that could cause an infinite loop
    // since it receives all such logs
    while let Some(mut logs) = rx.next().await {
        let timestamp = Utc::now();
        logs.iter_mut().for_each(|log| {
            log.insert_field(host_key.as_str(), hostname.as_str());
            log.insert_field(pid_key.as_str(), pid);
            log.insert_field(log_schema().source_type_key(), "internal_log");
            log.insert_field(log_schema().timestamp_key(), timestamp);
        });

        if let Err(err) = output.send_batch(logs).await {
            error!(message = "Error sending log", %err);
            return Err(());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use event::log::Value;
    use event::LogRecord;
    use futures::Stream;
    use testify::collect_ready;
    use tokio::time::sleep;

    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    #[tokio::test]
    async fn receive_logs() {
        let test_id: u8 = rand::random();
        let start = Utc::now();
        framework::trace::init(false, false, "debug", 10);
        framework::trace::reset_early_buffer();
        error!(message = "Before source started", %test_id);

        let rx = start_source().await;
        error!(message = "After source started", %test_id);

        sleep(Duration::from_millis(10)).await;
        let mut logs = collect_ready(rx).await;
        let test_id = Value::from(test_id.to_string());
        logs.retain(|log| log.get_field("test_id") == Some(&test_id));

        let end = Utc::now();

        assert_eq!(logs.len(), 2);
        assert_eq!(
            logs[0].get_field("message").unwrap(),
            &Value::from("Before source started")
        );
        assert_eq!(
            logs[1].get_field("message").unwrap().to_string_lossy(),
            "After source started"
        );

        for log in logs {
            let timestamp = log
                .get_field("timestamp")
                .unwrap()
                .as_timestamp()
                .expect("timestamp isn't a timestamp");
            assert!(*timestamp >= start);
            assert!(*timestamp <= end);
        }
    }

    async fn start_source() -> impl Stream<Item = LogRecord> {
        let (tx, rx) = Pipeline::new_test();
        let source = Config::default()
            .build(SourceContext::new_test(tx))
            .await
            .unwrap();

        tokio::spawn(source);
        sleep(Duration::from_millis(10)).await;
        framework::trace::stop_buffering();
        rx.map(|item| item.into_log())
    }
}
