use buffer::LimitedReceiver;
use configurable::configurable_component;
use event::{Events, Finalizable, MetricValue, log::Value};
use framework::OutputBuffer;
use framework::config::{
    DataType, InputType, OutputType, SinkConfig, SinkContext, SourceConfig, SourceContext,
    TransformConfig, TransformContext,
};
use framework::pipeline::Pipeline;
use framework::{FunctionTransform, Healthcheck, Sink, Source, StreamSink, Transform};
use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use log_schema::log_schema;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::sync::oneshot;
use tracing::{error, info};

#[configurable_component(source, name = "mock")]
pub struct MockSourceConfig {
    #[serde(skip)]
    receiver: Arc<Mutex<Option<LimitedReceiver<Events>>>>,
    #[serde(skip)]
    event_counter: Option<Arc<AtomicUsize>>,
    #[serde(skip)]
    data_type: Option<DataType>,
    #[serde(skip)]
    force_shutdown: bool,

    // something for serde to use, so we can trigger rebuilds
    data: Option<String>,
}

impl MockSourceConfig {
    pub fn new(receiver: LimitedReceiver<Events>) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(Some(receiver))),
            event_counter: None,
            data_type: Some(DataType::All),
            force_shutdown: false,
            data: None,
        }
    }

    pub fn new_with_data(receiver: LimitedReceiver<Events>, data: &str) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(Some(receiver))),
            event_counter: None,
            data_type: Some(DataType::All),
            force_shutdown: false,
            data: Some(data.into()),
        }
    }

    pub fn new_with_event_counter(force_shutdown: bool) -> (Pipeline, Self, Arc<AtomicUsize>) {
        let event_counter = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = Pipeline::new_test();

        (
            tx,
            Self {
                receiver: Arc::new(Mutex::new(Some(rx))),
                event_counter: Some(Arc::clone(&event_counter)),
                data_type: Some(DataType::All),
                force_shutdown,
                data: None,
            },
            event_counter,
        )
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "mock")]
impl SourceConfig for MockSourceConfig {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let wrapped = Arc::clone(&self.receiver);
        let event_counter = self.event_counter.clone();
        let mut recv = wrapped.lock().unwrap().take().unwrap();
        let shutdown1 = cx.shutdown.clone();
        let shutdown2 = cx.shutdown;
        let mut output = cx.output;
        let force_shutdown = self.force_shutdown;

        Ok(Box::pin(async move {
            tokio::pin!(shutdown1);
            tokio::pin!(shutdown2);

            loop {
                tokio::select! {
                    biased;

                    _ = &mut shutdown1, if force_shutdown => break,

                    Some(array) = recv.next() => {
                        if let Some(counter) = &event_counter {
                            counter.fetch_add(array.len(), Ordering::Relaxed);
                        }

                        if let Err(e) = output.send(array).await {
                            error!(message = "Error sending in sink..", %e);
                            return Err(())
                        }
                    },

                    _ = &mut shutdown2, if !force_shutdown => break,
                }
            }

            info!("finished sending.");

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::new(self.data_type.unwrap_or(DataType::Metric))]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

#[configurable_component(transform, name = "mock")]
#[derive(Clone)]
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

#[async_trait::async_trait]
#[typetag::serde(name = "mock")]
impl TransformConfig for MockTransformConfig {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        Ok(Transform::function(MockTransform {
            suffix: self.suffix.clone(),
            increase: self.increase,
        }))
    }

    fn input(&self) -> InputType {
        InputType::all()
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::new(DataType::All)]
    }
}

#[derive(Clone, Debug)]
struct MockTransform {
    suffix: String,
    increase: f64,
}

impl FunctionTransform for MockTransform {
    fn transform(&mut self, output: &mut OutputBuffer, mut events: Events) {
        match &mut events {
            Events::Logs(logs) => logs.iter_mut().for_each(|log| {
                let v = log
                    .get(log_schema().message_key())
                    .unwrap()
                    .to_string_lossy();

                log.insert(
                    log_schema().message_key(),
                    Value::from(format!("{}{}", v, self.suffix)),
                );
            }),

            Events::Metrics(metrics) => metrics.iter_mut().for_each(|metric| {
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
            }),

            Events::Traces(traces) => traces.iter_mut().for_each(|trace| {
                trace.service = format!("{}{}", trace.service, self.suffix).into();
            }),
        }

        output.push(events);
    }
}

#[derive(Debug, Clone, Default)]
enum Mode {
    Normal(Pipeline),

    #[default]
    Dead,
}

#[configurable_component(sink, name = "mock")]
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

    pub fn new_with_data(sink: Pipeline, healthy: bool, data: &str) -> Self {
        Self {
            sink: Mode::Normal(sink),
            healthy,
            data: Some(data.into()),
        }
    }
}

#[derive(Debug, Error)]
enum HealthcheckError {
    #[error("unhealthy")]
    Unhealthy,
}

#[async_trait::async_trait]
#[typetag::serde(name = "mock")]
impl SinkConfig for MockSinkConfig {
    async fn build(&self, _cx: SinkContext) -> framework::Result<(Sink, Healthcheck)> {
        // If this sink is set to not be healthy, just send the healthcheck error
        // immediately over the oneshot.. otherwise, pass the sink so it can send
        // it only once it has started running, so that tests can request the topology
        // be healthy before proceeding.
        let (tx, rx) = oneshot::channel();
        let health_tx = if self.healthy {
            Some(tx)
        } else {
            let _ = tx.send(Err(HealthcheckError::Unhealthy.into()));
            None
        };

        let sink = MockSink {
            sink: self.sink.clone(),
            health_tx,
        };

        let healthcheck = async move { rx.await.unwrap() };

        Ok((Sink::Stream(Box::new(sink)), healthcheck.boxed()))
    }

    fn input_type(&self) -> InputType {
        InputType::all()
    }
}

struct MockSink {
    sink: Mode,
    health_tx: Option<oneshot::Sender<framework::Result<()>>>,
}

#[async_trait::async_trait]
impl StreamSink for MockSink {
    async fn run(mut self: Box<Self>, mut input: BoxStream<'_, Events>) -> Result<(), ()> {
        match self.sink {
            Mode::Normal(mut sink) => {
                if let Some(tx) = self.health_tx.take() {
                    let _ = tx.send(Ok(()));
                }

                // We have an inner sink, so forward the input normally
                while let Some(mut events) = input.next().await {
                    let finalizers = events.take_finalizers();
                    if let Err(err) = sink.send(events).await {
                        error!(
                            message = "Ingesting events failed at mock sink",
                            %err
                        )
                    }

                    drop(finalizers)
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
