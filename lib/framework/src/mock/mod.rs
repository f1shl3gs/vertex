use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;

use async_trait::async_trait;
use buffers::Acker;
use event::{Event, MetricValue, Value};
use futures::{FutureExt, StreamExt};
use futures_util::stream;
use futures_util::stream::BoxStream;
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tracing::{error, info};

use crate::config::{
    DataType, Output, SinkConfig, SinkContext, SourceConfig, SourceContext, TransformConfig,
    TransformContext,
};
use crate::pipeline::{Pipeline, ReceiverStream};
use crate::{FunctionTransform, Healthcheck, Sink, Source, StreamSink, Transform};

#[derive(Debug, Deserialize, Serialize)]
pub struct MockSourceConfig {
    #[serde(skip)]
    receiver: Arc<Mutex<Option<ReceiverStream<Event>>>>,
    #[serde(skip)]
    event_counter: Option<Arc<AtomicUsize>>,
    #[serde(skip)]
    data_type: Option<DataType>,

    // something for serde to use, so we can trigger rebuilds
    data: Option<String>,
}

impl MockSourceConfig {
    pub fn new(receiver: ReceiverStream<Event>) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(Some(receiver))),
            event_counter: None,
            data_type: Some(DataType::Any),
            data: None,
        }
    }

    pub fn new_with_data(receiver: ReceiverStream<Event>, data: &str) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(Some(receiver))),
            event_counter: None,
            data_type: Some(DataType::Any),
            data: Some(data.into()),
        }
    }

    pub fn new_with_event_counter(
        receiver: ReceiverStream<Event>,
        event_counter: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(Some(receiver))),
            event_counter: Some(event_counter),
            data_type: Some(DataType::Any),
            data: None,
        }
    }
}

#[async_trait]
#[typetag::serde(name = "mock")]
impl SourceConfig for MockSourceConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let wrapped = self.receiver.clone();
        let event_counter = self.event_counter.clone();
        let mut recv = wrapped.lock().unwrap().take().unwrap();
        let mut shutdown = Some(cx.shutdown);
        let mut output = cx.output;

        Ok(Box::pin(async move {
            let mut stream = stream::poll_fn(move |cx| {
                if let Some(until) = shutdown.as_mut() {
                    match until.poll_unpin(cx) {
                        Poll::Ready(_res) => {
                            shutdown.take();
                            recv.close();
                        }

                        Poll::Pending => {}
                    }
                }

                recv.poll_next_unpin(cx)
            })
            .inspect(move |_| {
                if let Some(counter) = &event_counter {
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            });

            match output.send_all(&mut stream).await {
                Ok(()) => {
                    info!(message = "Finished sending");
                    Ok(())
                }
                Err(err) => {
                    error!(message = "Error sending in sink", %err);
                    Err(())
                }
            }
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(self.data_type.unwrap_or(DataType::Metric))]
    }

    fn source_type(&self) -> &'static str {
        "mock"
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MockTransformConfig {
    #[serde(default)]
    suffix: String,
    #[serde(default)]
    increase: f64,
}

impl MockTransformConfig {
    pub fn new(suffix: String, increase: f64) -> Self {
        Self { suffix, increase }
    }
}

#[async_trait]
#[typetag::serde(name = "mock")]
impl TransformConfig for MockTransformConfig {
    async fn build(&self, _ctx: &TransformContext) -> crate::Result<Transform> {
        Ok(Transform::function(MockTransform {
            suffix: self.suffix.clone(),
            increase: self.increase,
        }))
    }

    fn input_type(&self) -> DataType {
        DataType::Any
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Any)]
    }

    fn transform_type(&self) -> &'static str {
        "mock"
    }
}

#[derive(Clone, Debug)]
struct MockTransform {
    suffix: String,
    increase: f64,
}

impl FunctionTransform for MockTransform {
    fn transform(&mut self, output: &mut Vec<Event>, mut event: Event) {
        match &mut event {
            Event::Log(log) => {
                let mut v = log
                    .get_field(log_schema().message_key())
                    .unwrap()
                    .to_string_lossy();

                v.push_str(&self.suffix);
                log.insert_field(log_schema().message_key(), Value::from(v));
            }

            Event::Metric(metric) => {
                let value = match &metric.value {
                    MetricValue::Sum(v) => MetricValue::Sum(*v + self.increase),
                    MetricValue::Gauge(v) => MetricValue::Gauge(*v + self.increase),
                    MetricValue::Histogram {
                        count,
                        sum,
                        buckets,
                    } => MetricValue::Histogram {
                        count: count + 1,
                        sum: sum + self.increase,
                        buckets: buckets.clone(),
                    },
                    MetricValue::Summary {
                        count,
                        sum,
                        quantiles,
                    } => MetricValue::Summary {
                        count: count + 1,
                        sum: sum + self.increase,
                        quantiles: quantiles.clone(),
                    },
                };

                metric.value = value
            }

            Event::Trace(trace) => trace.service = format!("{}{}", trace.service, self.suffix),
        }

        output.push(event);
    }
}

#[derive(Debug, Clone)]
enum Mode {
    Normal(Pipeline),
    Dead,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Dead
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MockSinkConfig {
    #[serde(skip)]
    sink: Mode,
    #[serde(skip)]
    healthy: bool,
    // something for serde to use, so we can trigger rebuilds
    data: Option<String>,
}

impl MockSinkConfig {
    pub fn new(sink: Pipeline, healthy: bool) -> Self {
        Self {
            sink: Mode::Normal(sink),
            healthy,
            data: None,
        }
    }
}

#[derive(Debug, Snafu)]
enum HealthcheckError {
    #[snafu(display("unhealthy"))]
    Unhealthy,
}

#[async_trait]
#[typetag::serde(name = "mock")]
impl SinkConfig for MockSinkConfig {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let sink = MockSink {
            acker: cx.acker(),
            sink: self.sink.clone(),
        };

        let healthcheck = if self.healthy {
            futures::future::ok(())
        } else {
            futures::future::err(HealthcheckError::Unhealthy.into())
        };

        Ok((crate::Sink::Stream(Box::new(sink)), healthcheck.boxed()))
    }

    fn input_type(&self) -> DataType {
        DataType::Any
    }

    fn sink_type(&self) -> &'static str {
        "mock"
    }
}

struct MockSink {
    acker: Acker,
    sink: Mode,
}

#[async_trait]
impl StreamSink for MockSink {
    async fn run(self: Box<Self>, mut input: BoxStream<'_, Event>) -> Result<(), ()> {
        match self.sink {
            Mode::Normal(mut sink) => {
                // We have an inner sink, so forward the input normally
                while let Some(event) = input.next().await {
                    if let Err(err) = sink.send(event).await {
                        error!(
                            message = "ingesting an event failed at mock sink",
                            %err
                        );
                    }

                    self.acker.ack(1);
                }
            }

            Mode::Dead => {
                // Simulate a dead sink and never poll the input
                futures::future::pending::<()>().await;
            }
        }

        Ok(())
    }
}
