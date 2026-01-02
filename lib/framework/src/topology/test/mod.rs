// mod backpressure;
mod doesnt_reload;
mod reload;
mod source_finished;
mod transient_state;

mod utils;

use std::net::SocketAddr;
use std::time::Duration;

use configurable::configurable_component;
use event::{Events, LogRecord};
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use tokio::task::JoinSet;
pub use utils::start_topology;

use crate::config::{
    InputType, OutputType, Protocol, Resource, SinkConfig, SinkContext, SourceConfig,
    SourceContext, TransformConfig, TransformContext,
};
use crate::source::Source;
use crate::{FunctionTransform, Healthcheck, OutputBuffer, Sink, StreamSink, Transform};

#[configurable_component(source, name = "generate_log")]
#[derive(Clone)]
pub struct GenerateLogSource {
    count: usize,
}

impl GenerateLogSource {
    pub fn new(count: usize) -> GenerateLogSource {
        GenerateLogSource { count }
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "type")]
impl SourceConfig for GenerateLogSource {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let count = self.count;
        let mut shutdown = cx.shutdown;
        let mut output = cx.output;

        Ok(Box::pin(async move {
            for _ in 0..count {
                let log = LogRecord::from("abcd");
                if let Err(_err) = output.send(log).await {
                    break;
                }

                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(10)) => {},
                    _ = &mut shutdown => break
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::log()]
    }
}

#[configurable_component(source, name = "resource")]
pub struct ResourceSourceConfig {
    pub listen: SocketAddr,
}

impl ResourceSourceConfig {
    pub fn new(listen: SocketAddr) -> ResourceSourceConfig {
        ResourceSourceConfig { listen }
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "resource")]
impl SourceConfig for ResourceSourceConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let mut shutdown = cx.shutdown;
        let listen = tokio::net::TcpListener::bind(&self.listen).await?;

        Ok(Box::pin(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown => break,
                    _ = listen.accept() => {}
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::log()]
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::Port(self.listen, Protocol::Tcp)]
    }
}

#[configurable_component(transform, name = "noop")]
#[derive(Clone)]
pub struct NoopTransformConfig;

#[async_trait::async_trait]
#[typetag::serde(name = "noop")]
impl TransformConfig for NoopTransformConfig {
    async fn build(&self, _cx: &TransformContext) -> crate::Result<Transform> {
        Ok(Transform::Function(Box::new(NoopTransform)))
    }

    fn input(&self) -> InputType {
        InputType::all()
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::log()]
    }
}

#[derive(Clone)]
struct NoopTransform;

impl FunctionTransform for NoopTransform {
    fn transform(&mut self, output: &mut OutputBuffer, events: Events) {
        output.push(events);
    }
}

#[configurable_component(sink, name = "null")]
pub struct NullSinkConfig;

#[async_trait::async_trait]
#[typetag::serde(name = "null")]
impl SinkConfig for NullSinkConfig {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let sink = NullSink;
        let healthcheck = futures::future::ok(()).boxed();

        Ok((Sink::Stream(Box::new(sink)), healthcheck))
    }

    fn input_type(&self) -> InputType {
        InputType::log()
    }
}

struct NullSink;

#[async_trait::async_trait]
impl StreamSink for NullSink {
    async fn run(self: Box<Self>, mut input: BoxStream<'_, Events>) -> Result<(), ()> {
        while let Some(events) = input.next().await {
            drop(events);
        }

        Ok(())
    }
}

#[configurable_component(sink, name = "mock")]
pub struct MockSinkConfig {
    #[configurable(skip)]
    #[serde(skip)]
    resources: Vec<Resource>,
}

impl MockSinkConfig {
    pub fn tcp(addr: SocketAddr) -> MockSinkConfig {
        MockSinkConfig {
            resources: vec![Resource::Port(addr, Protocol::Tcp)],
        }
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "mock")]
impl SinkConfig for MockSinkConfig {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let healthcheck = futures::future::ok(()).boxed();

        if self.resources.is_empty() {
            Ok((Sink::Stream(Box::new(NullSink)), healthcheck))
        } else {
            let fut = alloc_resources(&self.resources).await?;
            Ok((Sink::Stream(Box::new(MockSink { fut })), healthcheck))
        }
    }

    fn input_type(&self) -> InputType {
        InputType::log()
    }

    fn resources(&self) -> Vec<Resource> {
        self.resources.clone()
    }
}

struct MockSink {
    fut: BoxFuture<'static, ()>,
}

#[async_trait::async_trait]
impl StreamSink for MockSink {
    async fn run(mut self: Box<Self>, mut input: BoxStream<'_, Events>) -> Result<(), ()> {
        loop {
            tokio::select! {
                _ = input.next() => {},
                _ = &mut self.fut => break,
            }
        }

        Ok(())
    }
}

async fn alloc_resources(resources: &[Resource]) -> crate::Result<BoxFuture<'static, ()>> {
    let mut tasks = JoinSet::new();

    for resource in resources {
        match resource {
            Resource::Port(addr, Protocol::Tcp) => {
                let listener = tokio::net::TcpListener::bind(addr).await?;

                tasks.spawn(async move {
                    loop {
                        _ = listener.accept();
                    }
                });
            }
            Resource::Port(addr, Protocol::Udp) => {
                let listener = tokio::net::UdpSocket::bind(addr).await?;

                tasks.spawn(async move {
                    let mut buf = [0; 1024];

                    loop {
                        _ = listener.recv_from(&mut buf);
                    }
                });
            }
            _ => {
                println!("Unexpected resource type: {:?}", resource);
            }
        }
    }

    Ok(Box::pin(async move {
        while tasks.join_next().await.is_some() {}
    }))
}
