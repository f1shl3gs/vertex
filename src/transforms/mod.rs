#[cfg(feature = "transforms-add_fields")]
mod add_fields;
#[cfg(feature = "transforms-add_tags")]
mod add_tags;
mod aggregate;
mod ansii_striper;

#[cfg(feature = "transforms-cardinality")]
mod cardinality;

mod filter;
mod geoip;
mod grok_parser;
mod jsmn_parser;
mod json_parser;
mod logfmt_parser;
pub mod noop;
mod rename_fields;
mod rename_tags;
mod route;
#[cfg(feature = "transforms-sample")]
mod sample;

use crate::config::Output;
use crate::topology::{ControlChannel, Fanout};
use event::Event;
use futures::Stream;
use futures_util::SinkExt;
use shared::ByteSizeOf;
use std::collections::HashMap;
use std::pin::Pin;

/// Transforms that are simple, and don't require attention to coordination.
/// You can run them as simple functions over events in any order
pub trait FunctionTransform: Send + dyn_clone::DynClone + Sync {
    fn transform(&mut self, output: &mut Vec<Event>, event: Event);
}

dyn_clone::clone_trait_object!(FunctionTransform);

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
        task: Pin<Box<dyn Stream<Item = Event> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = Event> + Send>>;
}

/// Broader than the simple [`FunctionTransform`], this trait allows transforms
/// to write to multiple outputs. Those outputs must be known in advanced and
/// returned via `TransformConfig::outputs`. Attempting to send to any output
/// not registered in advance is considered a bug and will cause a panic.
pub trait SyncTransform: Send + dyn_clone::DynClone + Sync {
    fn transform(&mut self, event: Event, output: &mut TransformOutputsBuf);
}

dyn_clone::clone_trait_object!(SyncTransform);

impl<T> SyncTransform for T
where
    T: FunctionTransform,
{
    fn transform(&mut self, event: Event, output: &mut TransformOutputsBuf) {
        FunctionTransform::transform(
            self,
            output.primary_buffer.as_mut().expect("no default output"),
            event,
        );
    }
}

// TODO: this is a bit ugly when we already have the above impl
impl SyncTransform for Box<dyn FunctionTransform> {
    fn transform(&mut self, event: Event, output: &mut TransformOutputsBuf) {
        FunctionTransform::transform(
            self.as_mut(),
            output.primary_buffer.as_mut().expect("no default output"),
            event,
        );
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
            send_inner(
                buf.primary_buffer.as_mut().expect("mismatched outputs"),
                primary,
            )
            .await;
        }

        for (key, buf) in &mut buf.named_buffers {
            send_inner(
                buf,
                self.named_outputs.get_mut(key).expect("unknown output"),
            )
            .await;
        }
    }
}

async fn send_inner(buf: &mut Vec<Event>, output: &mut Fanout) {
    for event in buf.drain(..) {
        output.feed(event).await.expect("unit error")
    }

    output.flush().await.expect("unit error")
}

pub struct TransformOutputsBuf {
    primary_buffer: Option<Vec<Event>>,
    named_buffers: HashMap<String, Vec<Event>>,
}

impl TransformOutputsBuf {
    pub fn new_with_capacity(outpus: Vec<Output>, capacity: usize) -> Self {
        let mut primary_buffer = None;
        let mut named_buffers = HashMap::new();

        for output in outpus {
            match output.port {
                Some(name) => {
                    named_buffers.insert(name.clone(), vec![]);
                }
                None => {
                    primary_buffer = Some(Vec::with_capacity(capacity));
                }
            }
        }

        Self {
            primary_buffer,
            named_buffers,
        }
    }

    pub fn push(&mut self, event: Event) {
        self.primary_buffer
            .as_mut()
            .expect("no default output")
            .push(event);
    }

    pub fn push_named(&mut self, name: &str, event: Event) {
        self.named_buffers
            .get_mut(name)
            .expect("unknown output")
            .push(event);
    }

    pub fn append(&mut self, slice: &mut Vec<Event>) {
        self.primary_buffer
            .as_mut()
            .expect("no default output")
            .append(slice);
    }

    pub fn append_named(&mut self, name: &str, slice: &mut Vec<Event>) {
        self.named_buffers
            .get_mut(name)
            .expect("unknown output")
            .append(slice);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = Event> + '_ {
        self.primary_buffer
            .as_mut()
            .expect("no default output")
            .drain(..)
    }

    pub fn drain_named(&mut self, name: &str) -> impl Iterator<Item = Event> + '_ {
        self.named_buffers
            .get_mut(name)
            .expect("unknown output")
            .drain(..)
    }

    pub fn take_primary(&mut self) -> Vec<Event> {
        std::mem::take(self.primary_buffer.as_mut().expect("no default output"))
    }

    pub fn take_all_nmaed(&mut self) -> HashMap<String, Vec<Event>> {
        std::mem::take(&mut self.named_buffers)
    }

    pub fn len(&self) -> usize {
        self.primary_buffer.as_ref().map_or(0, Vec::len)
            + self
                .named_buffers
                .iter()
                .map(|(_, buf)| buf.len())
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
                .iter()
                .map(|(_, buf)| buf.size_of())
                .sum::<usize>()
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
}

/// Transform a single `Event` through the `FunctionTransform`
///
/// # Panics
///
/// If `ft` attempts to emit more than one `Event` on transform this function
/// will panic.
#[cfg(test)]
pub fn transform_one(
    ft: &mut dyn FunctionTransform,
    event: impl Into<event::Event>,
) -> Option<Event> {
    let mut buf = Vec::with_capacity(1);
    ft.transform(&mut buf, event.into());
    assert!(buf.len() < 2);
    buf.into_iter().next()
}
