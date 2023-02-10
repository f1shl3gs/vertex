use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::time::Duration;

use configurable::Configurable;
use event::EventFinalizers;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::GenerateConfig;
use crate::stream::BatcherSettings;

// Provide sensible sink default 10MB with 1s timeout.
// Don't allow chaining builder methods on that.

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Error, PartialEq)]
pub enum BatchError {
    #[error("This sink does not allow setting `max_bytes`")]
    BytesNotAllowed,
    #[error("`max_bytes` must be greater than zero")]
    InvalidMaxBytes,
    #[error("`max_events` must be greater than zero")]
    InvalidMaxEvents,
    #[error("`timeout` must be greater than zero")]
    InvalidTimeout,
    #[error("provided `max_bytes` exceeds the maximum limit of {0}")]
    MaxBytesExceeded(usize),
    #[error("provided `max_events` exceeds the maximum limit of {0}")]
    MaxEventsExceeded(usize),
}

#[derive(Debug)]
pub struct BatchSize<B> {
    pub bytes: usize,
    pub events: usize,
    // this type marker is used to drive type inference, which allows us to
    // call the right Batch::get_settings_defaults without explicitly naming
    // the type in BatchSettings::parse_config
    _b: PhantomData<B>,
}

impl<B> Clone for BatchSize<B> {
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes,
            events: self.events,
            _b: PhantomData,
        }
    }
}

impl<B> Copy for BatchSize<B> {}

impl<B> BatchSize<B> {
    pub const fn const_default() -> Self {
        BatchSize {
            bytes: usize::MAX,
            events: usize::MAX,
            _b: PhantomData,
        }
    }
}

impl<B> Default for BatchSize<B> {
    fn default() -> Self {
        BatchSize::const_default()
    }
}

#[derive(Debug)]
pub struct BatchSettings<B> {
    pub size: BatchSize<B>,
    pub timeout: std::time::Duration,
}

impl<B> Default for BatchSettings<B> {
    fn default() -> Self {
        BatchSettings {
            size: BatchSize {
                bytes: 10_000_000,
                events: usize::MAX,
                _b: PhantomData,
            },
            timeout: std::time::Duration::from_secs(1),
        }
    }
}

/// This enum provides the result of a push operation, indicating if the
/// event was added and the fullness state of the buffer.
#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub enum PushResult<T> {
    /// Event was added, with an indicator if the buffer is now full
    Ok(bool),
    /// Event could not be added because it would overflow the buffer.
    /// Since push takes ownership of the event, it must be returned here.
    Overflow(T),
}

pub fn err_event_too_large<T>(length: usize, max_length: usize) -> PushResult<T> {
    error!(
        message = "Event larger than batch max_bytes; dropping event.",
        batch_max_bytes = %max_length,
        length = %length,
        internal_log_rate_secs = 1
    );

    metrics::register_counter(
        "events_discarded_total",
        "The total number of events discarded by this component.",
    )
    .recorder(&[("reason", "oversized")])
    .inc(1);

    PushResult::Ok(false)
}

pub trait SinkBatchSettings {
    const MAX_EVENTS: Option<usize>;
    const MAX_BYTES: Option<usize>;

    const TIMEOUT: Duration;
}

/// Reasonable default batch settings for sinks with timeliness concerns,
/// limit by event count.
#[derive(Clone, Copy, Debug, Default)]
pub struct RealtimeEventBasedDefaultBatchSettings;

impl SinkBatchSettings for RealtimeEventBasedDefaultBatchSettings {
    const MAX_EVENTS: Option<usize> = Some(1000);
    const MAX_BYTES: Option<usize> = None;
    const TIMEOUT: Duration = Duration::from_secs(1);
}

/// Reasonable default batch settings for sinks with timeliness concerns,
/// limited by byte size.
#[derive(Clone, Copy, Debug, Default)]
pub struct RealtimeSizeBasedDefaultBatchSettings;

impl SinkBatchSettings for RealtimeSizeBasedDefaultBatchSettings {
    const MAX_EVENTS: Option<usize> = None;
    const MAX_BYTES: Option<usize> = Some(10_000_000);
    const TIMEOUT: Duration = Duration::from_secs(1);
}

/// Reasonable default batch settings for sinks focused on shipping
/// fewer-but-larger batches, limit by byte size.
#[derive(Clone, Copy, Debug, Default)]
pub struct BulkSizeBasedDefaultBatchSettings;

impl SinkBatchSettings for BulkSizeBasedDefaultBatchSettings {
    const MAX_EVENTS: Option<usize> = None;
    const MAX_BYTES: Option<usize> = Some(10_000_000);
    const TIMEOUT: Duration = Duration::from_secs(300);
}

/// "Default" batch settings when a sink handles batch settings
/// entirely on its own.
///
/// This has very few usages, but can be notably seen in the Kafka
/// sink, where the values are used to configure `librdkafka` itself
/// rather than being passed as `BatchSettings`/`BatcherSettings` to
/// components in the sink itself.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoDefaultBatchSettings;

impl SinkBatchSettings for NoDefaultBatchSettings {
    const MAX_EVENTS: Option<usize> = None;
    const MAX_BYTES: Option<usize> = None;
    const TIMEOUT: Duration = Duration::from_secs(1);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Merged;

#[derive(Clone, Copy, Debug, Default)]
pub struct Unmerged;

/// Configures the sink batching behavior.
#[derive(Configurable, Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct BatchConfig<D: SinkBatchSettings, S = Unmerged> {
    /// The maximum size of a batch, before it is flushed.
    #[serde(with = "humanize::bytes::serde_option")]
    pub max_bytes: Option<usize>,

    /// The maximum events of a batch, before it is flushed.
    pub max_events: Option<usize>,

    /// The maximum age of a batch before it is flushed.
    #[serde(with = "humanize::duration::serde_option")]
    pub timeout: Option<Duration>,

    #[serde(skip)]
    _d: PhantomData<D>,
    #[serde(skip)]
    _s: PhantomData<S>,
}

impl<D, S> GenerateConfig for BatchConfig<D, S>
where
    D: SinkBatchSettings,
{
    fn generate_config() -> String {
        r#"
# The maximum size of a batch, before it is flushed
#
# max_bytes: 4M

# The maximum size of a batch, before it is flushed.
#
# max_events: 1024

"#
        .into()
    }
}

impl<D: SinkBatchSettings> BatchConfig<D, Unmerged> {
    pub fn validate(self) -> Result<BatchConfig<D, Merged>, BatchError> {
        let timeout = D::TIMEOUT;
        let config = BatchConfig {
            max_bytes: self.max_bytes.or(D::MAX_BYTES),
            max_events: self.max_events.or(D::MAX_EVENTS),
            timeout: self.timeout.or(Some(timeout)),
            _d: PhantomData,
            _s: PhantomData,
        };

        match (config.max_bytes, config.max_events, config.timeout) {
            // TODO: what logic do we want to check that we have the minimum number of settings?
            //   for example, we always assert that timeout from D is greater than zero, but
            //   technically we could end up with max bytes or max events being none, since we
            //   just chain options... but asserting that they're set isn't really doable either,
            //   because you don't always set both of those fields, etc...
            (Some(0), _, _) => Err(BatchError::InvalidMaxBytes),
            (_, Some(0), _) => Err(BatchError::InvalidMaxEvents),
            (_, _, Some(timeout)) if timeout.is_zero() => Err(BatchError::InvalidTimeout),
            _ => Ok(config),
        }
    }

    pub fn into_batch_settings<T: Batch>(self) -> Result<BatchSettings<T>, BatchError> {
        let config = self.validate()?;
        config.into_batch_settings()
    }

    /// Converts these settings into `batcherSettings`.
    ///
    /// `BatcherSettings` is effectively the `vertex` spiritual successor
    /// of `BatchSettings<B>`. Once all sinks are rewritten in the new
    /// stream-based style and we can eschew customized batch buffer types,
    /// we can de-genericify `BatchSettings` and move it into a mod, and
    /// use that instead of `BatcherSettings`.
    pub fn into_batcher_settings(self) -> Result<BatcherSettings, BatchError> {
        let config = self.validate()?;
        config.into_batcher_settings()
    }
}

impl<D: SinkBatchSettings> BatchConfig<D, Merged> {
    pub fn validate(self) -> Result<BatchConfig<D, Merged>, BatchError> {
        Ok(self)
    }

    pub fn disallow_max_bytes(self) -> Result<Self, BatchError> {
        // Sinks that used `max_size` for an event count cannot count
        // bytes, so err if `max_bytes` is set.
        match self.max_bytes {
            Some(_) => Err(BatchError::BytesNotAllowed),
            None => Ok(self),
        }
    }

    pub fn limit_max_bytes(self, limit: usize) -> Result<Self, BatchError> {
        match self.max_bytes {
            Some(n) if n > limit => Err(BatchError::MaxBytesExceeded(limit)),
            _ => Ok(self),
        }
    }

    pub fn into_batch_settings<T: Batch>(self) -> Result<BatchSettings<T>, BatchError> {
        let adjusted = T::default_settings(self)?;

        // This is unfortunate since we technically have already made sure this
        // isn't possible in `validate`, but alas.
        let timeout = adjusted.timeout.ok_or(BatchError::InvalidTimeout)?;

        Ok(BatchSettings {
            size: BatchSize {
                bytes: adjusted.max_bytes.unwrap_or(usize::MAX),
                events: adjusted.max_events.unwrap_or(usize::MAX),
                _b: PhantomData,
            },
            timeout,
        })
    }

    /// Convert these settings into `BatcherSettings`
    ///
    /// `BatcherSettings` is effectively the vertex spiritual successor of `BatchSettings<B>`.
    /// Once all sinks are rewritten in the new stream-based style and we can eschew
    /// custom batch buffer types, we can de-genericify `BatchSettings` and move it into
    /// a mod, and use that instead of `BatchSettings`.
    pub fn into_batcher_settings(self) -> Result<BatcherSettings, BatchError> {
        let max_bytes = self
            .max_bytes
            .and_then(NonZeroUsize::new)
            .or_else(|| NonZeroUsize::new(usize::MAX))
            .expect("`max_bytes` should already be validated");
        let max_events = self
            .max_events
            .and_then(NonZeroUsize::new)
            .or_else(|| NonZeroUsize::new(usize::MAX))
            .expect("`max_bytes` should already be validated");

        // This is unfortunate since we technically have already made sure
        // that isn't possible in `validate`, but alas.
        let timeout = self.timeout.ok_or(BatchError::InvalidTimeout)?;

        Ok(BatcherSettings::new(timeout, max_bytes, max_events))
    }
}

pub trait Batch: Sized {
    type Input;
    type Output;

    /// Turn the batch configuration into an actualized set of settings,
    /// and deal with the proper behavior of `max_size` and if `max_bytes`
    /// may be set. This is in the trait to ensure all batch buffers
    /// implement it.
    fn default_settings<D: SinkBatchSettings>(
        config: BatchConfig<D, Merged>,
    ) -> Result<BatchConfig<D, Merged>, BatchError> {
        Ok(config)
    }

    fn push(&mut self, item: Self::Input) -> PushResult<Self::Input>;
    fn is_empty(&self) -> bool;
    fn fresh(&self) -> Self;
    fn finish(self) -> Self::Output;
    fn num_items(&self) -> usize;
}

#[derive(Debug)]
pub struct EncodedEvent<I> {
    pub item: I,
    pub finalizers: EventFinalizers,
    pub byte_size: usize,
}

impl<I> EncodedEvent<I> {
    /// Create a trivial input with no metadata. This method will be
    /// removed when all sinks are converted
    pub fn new(item: I, byte_size: usize) -> Self {
        Self {
            item,
            byte_size,
            finalizers: Default::default(),
        }
    }

    // This should be:
    // ```impl<F, I: From<F>> From<EncodedEvent<F>> for EncodedEvent<I>```
    // however, the compiler rejects that due to conflicting implementations
    // of `From` due to the generic ```impl<T> From<T> for T```
    pub fn from<F>(that: EncodedEvent<F>) -> Self
    where
        I: From<F>,
    {
        Self {
            item: I::from(that.item),
            finalizers: that.finalizers,
            byte_size: that.byte_size,
        }
    }

    /// Remap the item using an adapter
    pub fn map<T>(self, f: impl Fn(I) -> T) -> EncodedEvent<T> {
        EncodedEvent {
            item: f(self.item),
            finalizers: self.finalizers,
            byte_size: self.byte_size,
        }
    }
}

#[derive(Debug)]
pub struct EncodedBatch<I> {
    pub items: I,
    pub finalizers: EventFinalizers,
    pub count: usize,
    pub byte_size: usize,
}

/// This is a batch construct that stores on set of event finalizers
/// alongside the batch itself.
#[derive(Clone, Debug)]
pub struct FinalizersBatch<B> {
    inner: B,
    finalizers: EventFinalizers,
    // The count of items inserted into this batch is distinct from
    // the number of items recorded by the inner batch, as that inner
    // count could be smaller due to aggregated item(ie metrics).
    count: usize,
    byte_size: usize,
}

impl<B: Batch> From<B> for FinalizersBatch<B> {
    fn from(inner: B) -> Self {
        Self {
            inner,
            finalizers: Default::default(),
            count: 0,
            byte_size: 0,
        }
    }
}

impl<B: Batch> Batch for FinalizersBatch<B> {
    type Input = EncodedEvent<B::Input>;
    type Output = EncodedBatch<B::Output>;

    fn default_settings<D: SinkBatchSettings>(
        config: BatchConfig<D, Merged>,
    ) -> Result<BatchConfig<D, Merged>, BatchError> {
        B::default_settings(config)
    }

    fn push(&mut self, item: Self::Input) -> PushResult<Self::Input> {
        let EncodedEvent {
            item,
            finalizers,
            byte_size,
        } = item;

        match self.inner.push(item) {
            PushResult::Ok(full) => {
                self.finalizers.merge(finalizers);
                self.count += 1;
                self.byte_size += byte_size;
                PushResult::Ok(full)
            }
            PushResult::Overflow(item) => PushResult::Overflow(EncodedEvent {
                item,
                finalizers,
                byte_size,
            }),
        }
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn fresh(&self) -> Self {
        Self {
            inner: self.inner.fresh(),
            finalizers: Default::default(),
            count: 0,
            byte_size: 0,
        }
    }

    fn finish(self) -> Self::Output {
        EncodedBatch {
            items: self.inner.finish(),
            finalizers: self.finalizers,
            count: self.count,
            byte_size: self.byte_size,
        }
    }

    fn num_items(&self) -> usize {
        self.inner.num_items()
    }
}

#[derive(Clone, Debug)]
pub struct StatefulBatch<B> {
    inner: B,
    was_full: bool,
}

impl<B: Batch> From<B> for StatefulBatch<B> {
    fn from(inner: B) -> Self {
        Self {
            inner,
            was_full: false,
        }
    }
}

impl<B> StatefulBatch<B> {
    pub const fn was_full(&self) -> bool {
        self.was_full
    }

    #[allow(clippy::missing_const_for_fn)] // const cannot run destructor
    pub fn into_inner(self) -> B {
        self.inner
    }
}

impl<B: Batch> Batch for StatefulBatch<B> {
    type Input = B::Input;
    type Output = B::Output;

    fn default_settings<D: SinkBatchSettings>(
        config: BatchConfig<D, Merged>,
    ) -> Result<BatchConfig<D, Merged>, BatchError> {
        B::default_settings(config)
    }

    fn push(&mut self, item: Self::Input) -> PushResult<Self::Input> {
        if self.was_full {
            PushResult::Overflow(item)
        } else {
            let result = self.inner.push(item);
            self.was_full =
                matches!(result, PushResult::Overflow(_)) || matches!(result, PushResult::Ok(true));

            result
        }
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn fresh(&self) -> Self {
        Self {
            inner: self.inner.fresh(),
            was_full: false,
        }
    }

    fn finish(self) -> Self::Output {
        self.inner.finish()
    }

    fn num_items(&self) -> usize {
        self.inner.num_items()
    }
}
