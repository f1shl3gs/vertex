use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;
use configurable::configurable_component;
use event::{Event, EventContainer, EventDataEq, EventRef, Events};
use futures::Stream;
use futures_util::{stream, StreamExt};
use measurable::ByteSizeOf;

use crate::config::{DataType, Output, TransformConfig, TransformContext};
use crate::topology::{ControlChannel, Fanout};

/// Transforms that are simple, and don't require attention to coordination.
/// You can run them as simple functions over events in any order
pub trait FunctionTransform: Send + dyn_clone::DynClone + Sync {
    fn transform(&mut self, output: &mut OutputBuffer, events: Events);
}

dyn_clone::clone_trait_object!(FunctionTransform);

/// Broader than the simple [`FunctionTransform`], this trait allows transforms
/// to write to multiple outputs. Those outputs must be known in advanced and
/// returned via `TransformConfig::outputs`. Attempting to send to any output
/// not registered in advance is considered a bug and will cause a panic.
pub trait SyncTransform: Send + dyn_clone::DynClone + Sync {
    fn transform(&mut self, events: Events, output: &mut TransformOutputsBuf);
}

dyn_clone::clone_trait_object!(SyncTransform);

impl<T> SyncTransform for T
where
    T: FunctionTransform,
{
    fn transform(&mut self, events: Events, output: &mut TransformOutputsBuf) {
        FunctionTransform::transform(
            self,
            output.primary_buffer.as_mut().expect("no default output"),
            events,
        );
    }
}

/// Transforms that tend to be more complicated runtime style componets.
///
/// These require coordination and map a stream of some `T` to some `U`
///
/// # Invariants
///
/// * It is an illegal invariant to implement `FunctionTransform` for a
/// `TaskTransform` or vice versa.
pub trait TaskTransform: Send {
    fn transform(
        self: Box<Self>,
        task: Pin<Box<dyn Stream<Item = Events> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = Events> + Send>>;

    /// Wrap the transform task to process and emit individual
    /// events. This is used to simplify testing task transforms.
    fn transform_events(
        self: Box<Self>,
        task: Pin<Box<dyn Stream<Item = Event> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = Event> + Send>> {
        self.transform(task.map(Into::into).boxed())
            .map(EventContainer::into_events)
            .flat_map(stream::iter)
            .boxed()
    }
}

// TODO: this is a bit ugly when we already have the above impl
impl SyncTransform for Box<dyn FunctionTransform> {
    fn transform(&mut self, events: Events, output: &mut TransformOutputsBuf) {
        FunctionTransform::transform(
            self.as_mut(),
            output.primary_buffer.as_mut().expect("no default output"),
            events,
        );
    }
}

/// Transforms come in two variants. Functions or tasks.
/// While function transforms can be run out of order or concurrently,
/// task transforms act as a coordination or barrier point.
pub enum Transform {
    Function(Box<dyn FunctionTransform>),
    Synchronous(Box<dyn SyncTransform>),
    Task(Box<dyn TaskTransform>),
}

impl Transform {
    /// Create a new function transform
    ///
    /// These functions are "stateless" and can be run in parallel,
    /// without regard for coordination.
    pub fn function(v: impl FunctionTransform + 'static) -> Self {
        Transform::Function(Box::new(v))
    }

    /// Create a new synchronous transform.
    ///
    /// This is a broader trait than the simple [`FunctionTransform`] in that it allows transforms
    /// to write to multiple outputs. Those outputs must be known in advanced and returned via
    /// `TransformConfig::outputs`. Attempting to send to any output not registered in advance is
    ///considered a bug and will cause a panic.
    pub fn synchronous(v: impl SyncTransform + 'static) -> Self {
        Transform::Synchronous(Box::new(v))
    }

    /// Mutably borrow the inner transform as a function transform.
    pub fn as_function(&mut self) -> &mut Box<dyn FunctionTransform> {
        match self {
            Transform::Function(t) => t,
            _ => panic!(
                "Called `Transform::as_function` on something that was not a function variant"
            ),
        }
    }

    /// Create a new task transform.
    ///
    /// These tasks are coordinated, and map a stream of some `U` to some other
    /// `T`.
    ///
    /// **Note:** You should prefer to implement [`FunctionTransform`] over this
    /// where possible.
    pub fn task(v: impl TaskTransform + 'static) -> Self {
        Transform::Task(Box::new(v))
    }

    /// Create a new task transform over individual `Event`s.
    ///
    /// These tasks are coordinated, and map a stream of some `U` to
    /// some other `T`.
    ///
    /// **Note:** You should prefer to implement `FunctionTransform`
    /// over this where possible.
    pub fn event_task(v: impl TaskTransform + 'static) -> Self {
        Transform::Task(Box::new(v))
    }

    /// Transmute the inner transform into a task transform.
    ///
    /// # Panics
    ///
    /// If the transform is not a `TaskTransform` this will panic.
    pub fn into_task(self) -> Box<dyn TaskTransform> {
        match self {
            Transform::Task(task) => task,
            _ => panic!("Called `into_task` on something that was not a task variant"),
        }
    }
}

pub struct TransformOutputsBuf {
    primary_buffer: Option<OutputBuffer>,
    named_buffers: HashMap<String, OutputBuffer>,
}

impl TransformOutputsBuf {
    pub fn new_with_capacity(outpus: Vec<Output>, capacity: usize) -> Self {
        let mut primary_buffer = None;
        let mut named_buffers = HashMap::new();

        for output in outpus {
            match output.port {
                Some(name) => {
                    named_buffers.insert(name.clone(), OutputBuffer::default());
                }
                None => {
                    primary_buffer = Some(OutputBuffer::with_capacity(capacity));
                }
            }
        }

        Self {
            primary_buffer,
            named_buffers,
        }
    }

    pub fn push_named(&mut self, name: &str, events: Events) {
        self.named_buffers
            .get_mut(name)
            .expect("unknown output")
            .push(events);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = Events> + '_ {
        self.primary_buffer
            .as_mut()
            .expect("no default output")
            .drain()
    }

    pub fn drain_named(&mut self, name: &str) -> impl Iterator<Item = Events> + '_ {
        self.named_buffers
            .get_mut(name)
            .expect("unknown output")
            .drain()
    }

    pub fn take_primary(&mut self) -> OutputBuffer {
        std::mem::take(self.primary_buffer.as_mut().expect("no default output"))
    }

    pub fn take_all_nmaed(&mut self) -> HashMap<String, OutputBuffer> {
        std::mem::take(&mut self.named_buffers)
    }

    pub fn len(&self) -> usize {
        self.primary_buffer.as_ref().map_or(0, OutputBuffer::len)
            + self
                .named_buffers
                .values()
                .map(|buf| buf.len())
                .sum::<usize>()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl ByteSizeOf for TransformOutputsBuf {
    fn allocated_bytes(&self) -> usize {
        self.primary_buffer.size_of()
            + self
                .named_buffers
                .values()
                .map(|buf| buf.size_of())
                .sum::<usize>()
    }
}

pub struct TransformOutputs {
    outputs_spec: Vec<Output>,
    primary_output: Option<Fanout>,
    named_outputs: HashMap<String, Fanout>,
}

impl TransformOutputs {
    pub fn new(outputs: Vec<Output>) -> (Self, HashMap<Option<String>, ControlChannel>) {
        let outputs_spec = outputs.clone();
        let mut primary_output = None;
        let mut named_outputs = HashMap::new();
        let mut controls = HashMap::new();

        for output in outputs {
            let (fanout, control) = Fanout::new();
            match output.port {
                None => {
                    primary_output = Some(fanout);
                    controls.insert(None, control);
                }
                Some(name) => {
                    named_outputs.insert(name.clone(), fanout);
                    controls.insert(Some(name.clone()), control);
                }
            }
        }

        (
            Self {
                outputs_spec,
                primary_output,
                named_outputs,
            },
            controls,
        )
    }

    pub fn new_buf_with_capacity(&self, capacity: usize) -> TransformOutputsBuf {
        TransformOutputsBuf::new_with_capacity(self.outputs_spec.clone(), capacity)
    }

    pub async fn send(&mut self, buf: &mut TransformOutputsBuf) {
        if let Some(primary) = self.primary_output.as_mut() {
            buf.primary_buffer
                .as_mut()
                .expect("mismatched outputs")
                .send(primary)
                .await;
        }

        for (key, buf) in &mut buf.named_buffers {
            buf.send(self.named_outputs.get_mut(key).expect("unknown output"))
                .await;
        }
    }
}

#[configurable_component(transform, name = "noop")]
#[derive(Clone, Debug, Default)]
pub struct Noop;

#[async_trait]
#[typetag::serde(name = "noop")]
impl TransformConfig for Noop {
    async fn build(&self, _cx: &TransformContext) -> crate::Result<Transform> {
        Ok(Transform::function(self.clone()))
    }

    fn input_type(&self) -> DataType {
        DataType::All
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::All)]
    }
}

impl FunctionTransform for Noop {
    fn transform(&mut self, output: &mut OutputBuffer, events: Events) {
        output.push(events);
    }
}

#[derive(Debug, Default, Clone)]
pub struct OutputBuffer(Vec<Events>);

impl OutputBuffer {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    #[inline]
    pub fn push(&mut self, events: impl Into<Events>) {
        self.0.push(events.into())
    }

    #[inline]
    pub fn push_one(&mut self, event: Event) {
        self.push_event(event)
    }

    fn push_event(&mut self, event: Event) {
        // Coalesce multiple pushes of the same type into one array.
        match (event, self.0.last_mut()) {
            (Event::Log(log), Some(Events::Logs(logs))) => {
                logs.push(log);
            }
            (Event::Metric(metric), Some(Events::Metrics(metrics))) => {
                metrics.push(metric);
            }
            (Event::Trace(trace), Some(Events::Traces(traces))) => {
                traces.push(trace);
            }
            (event, _) => {
                self.0.push(event.into());
            }
        }
    }

    pub fn extend(&mut self, events: impl Iterator<Item = Event>) {
        for event in events {
            self.push_event(event);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.iter().map(Events::len).sum()
    }

    pub fn first(&self) -> Option<EventRef> {
        self.0.first().and_then(|first| match first {
            Events::Logs(l) => l.first().map(Into::into),
            Events::Metrics(m) => m.first().map(Into::into),
            Events::Traces(t) => t.first().map(Into::into),
        })
    }

    pub fn drain(&mut self) -> impl Iterator<Item = Events> + '_ {
        self.0.drain(..)
    }

    async fn send(&mut self, output: &mut Fanout) {
        for array in std::mem::take(&mut self.0) {
            output.send(array).await;
        }
    }

    fn iter_events(&self) -> impl Iterator<Item = EventRef> {
        self.0.iter().flat_map(Events::iter_events)
    }

    pub fn into_events(self) -> impl Iterator<Item = Event> {
        self.0.into_iter().flat_map(Events::into_events)
    }
}

impl ByteSizeOf for OutputBuffer {
    fn allocated_bytes(&self) -> usize {
        self.0.iter().map(ByteSizeOf::size_of).sum()
    }
}

impl EventDataEq<Vec<Event>> for OutputBuffer {
    fn event_data_eq(&self, other: &Vec<Event>) -> bool {
        struct Comparator<'a>(EventRef<'a>);

        impl<'a> PartialEq<&Event> for Comparator<'a> {
            fn eq(&self, that: &&Event) -> bool {
                self.0.event_data_eq(that)
            }
        }

        self.iter_events().map(Comparator).eq(other.iter())
    }
}

impl From<Vec<Event>> for OutputBuffer {
    fn from(events: Vec<Event>) -> Self {
        let mut result = Self::default();
        result.extend(events.into_iter());
        result
    }
}
