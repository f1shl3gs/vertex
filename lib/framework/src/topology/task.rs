use buffers::channel::BufferReceiver;
use buffers::Acker;
use event::Event;
use futures::future::{BoxFuture, Future, FutureExt};
use pin_project::pin_project;
use std::pin::Pin;
use std::{
    fmt,
    task::{Context, Poll},
};

use crate::config::ComponentKey;
use crate::utilization::Utilization;

pub enum TaskOutput {
    Source,
    Transform,
    /// Buffer of sink
    Sink(Utilization<BufferReceiver<Event>>, Acker),
    HealthCheck,
    Extension,
}

/// High level topology task
#[pin_project]
pub struct Task {
    #[pin]
    inner: BoxFuture<'static, Result<TaskOutput, ()>>,
    key: ComponentKey,
    typetag: String,
}

impl Task {
    pub fn new<S, Fut>(key: ComponentKey, typetag: S, inner: Fut) -> Self
    where
        S: Into<String>,
        Fut: Future<Output = Result<TaskOutput, ()>> + Send + 'static,
    {
        Self {
            inner: inner.boxed(),
            key,
            typetag: typetag.into(),
        }
    }

    pub const fn key(&self) -> &ComponentKey {
        &self.key
    }

    pub fn typetag(&self) -> &str {
        &self.typetag
    }
}

impl Future for Task {
    type Output = Result<TaskOutput, ()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let task: &mut Task = self.get_mut();
        task.inner.as_mut().poll(cx)
    }
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("name", &self.key.id().to_string())
            .field("typetag", &self.typetag)
            .finish()
    }
}
