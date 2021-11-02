use std::collections::HashMap;
use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::Bytes;

use chrono::Duration;
use futures::Stream;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio_util::time::delay_queue::Key;
use tokio_util::time::DelayQueue;

/// The mode of operation of the line aggregator
#[derive(Debug, Hash, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// All consecutive lines matching this pattern are included in the group.
    /// The first line (the line that matched the start pattern) does not need
    /// to match the `ContinueThrough` pattern.
    /// This is useful in cases such as a Java stack trace, where some indicator
    /// in the line (such as leading whitespace) indicates that it is an extension of
    /// the proceeding line.
    ContinueThrough,

    /// All consecutive lines matching this pattern, plus one additional line, are
    /// included in the group. This is useful in cases where a log message ends with
    /// a continuation marker, such as a backslash, indicating that the following line
    /// is part of the same message.
    ContinuePast,

    /// All consecutive lines not matching this pattern are included in the group.
    /// This is useful where a log line contains a marker indicating that it begins
    /// a new message.
    HaltBefore,

    /// All consecutive lines, up to and including the first line matching this pattern,
    /// are included in the group. This is useful where a log line ends with a
    /// termination marker, such as a semicolon.
    HaltWith,
}

/// Configuration parameters of the line aggregator
#[derive(Debug, Clone)]
pub struct Config {
    /// Start pattern to look for as a beginning of the message
    pub start_pattern: Regex,
    /// Condition pattern to look for. Exact behavior is configured via `mode`
    pub condition_pattern: Regex,
    /// Mode of operation, specifies how the condition pattern is interpreted.
    pub mode: Mode,
    /// The maximum time to wait for the continuation. Once this timeout is reached,
    /// the buffered message is guaranteed to be flushed, even if incomplete
    pub timeout: Duration,
}

/// Line aggregator.
///
/// Provides a `Stream` implementation that reads lines from the `inner` stream
/// and yields aggregated lines.
#[pin_project(project = LineAggProj)]
pub struct LineAgg<T, K, C> {
    /// The stream from which we read the lines.
    #[pin]
    inner: T,

    /// The core line aggregation logic.
    logic: Logic<K, C>,

    /// Stashed lines. When line aggregation results in more than one line being emitted,
    /// we have to stash lines and return them into the stream after that before doing any
    /// other work
    stashed: Option<(K, Bytes, C)>,

    /// Duration queue. We switch to draining mode when we get `None` from the inner stream.
    /// In this mode we stop polling `inner` for new lines and just flush all the buffered data.
    draining: Option<Vec<(K, Bytes, C)>>,
}

/// Core line aggregation logic
///
/// Encapsulates the essential state and the core logic for the line aggregation algorithm
pub struct Logic<K, C> {
    /// Configuration parameters to use.
    config: Config,

    /// Line per key
    /// Key is usually a filename or other line source identifier.
    buffers: HashMap<K, (Key, Aggregate<C>)>,

    /// A queue of key timeouts
    timeouts: DelayQueue<K>,
}

impl<T, K, C> LineAgg<T, K, C>
    where
        T: Stream<Item=(K, Bytes, C)> + Unpin,
        K: Hash + Eq + Clone
{
    /// Create a new `LineAgg` using the specified `inner` stream and preconfigured `logic`
    pub fn new(inner: T, logic: Logic<K, C>) -> Self {
        Self {
            inner,
            logic,
            draining: None,
            stashed: None,
        }
    }
}

impl<T, K, C> Stream for LineAgg<T, K, C>
    where
        T: Stream<Item=(K, Bytes, C)> + Unpin,
        K: Hash + Eq + Clone,
{
    /// `K` - file name, or other line source,
    /// `Bytes` - the line data,
    /// `C` - the context related the line data
    type Item = (K, Bytes, C);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            // If we have a stashed line, process it before doing anything else.
            if let Some((src, line, context)) = this.stashed.take() {
                // Handle the stashed line. If the handler gave us something - return it,
                // otherwise restart the loop iteration to start anew. Handler could've stashed
                // another value, continuing to the new loop iteration handles that.
                if let Some(val) = Self::handle_line_and_stashing(&mut this, src, line, context) {
                    return Poll::Ready(Some(val));
                }

                continue;
            }

            // If we're in draining mode, short circuit here.
            if let Some(to_drain) = &mut this.draining {
                match to_drain.pop() {
                    Some(val) => Poll::Ready(Some(val)),
                    _ => Poll::Ready(None)
                }
            }

            match this.inner.poll_next_unpin(cx) {
                Poll::Ready(Some((src, line, context))) => {
                    // Handle the incoming line we got from `inner`. If the handler gave us
                    // something - return it, otherwise continue with the flow.
                    if let Some(val) = Self::handle_line_and_stashing(&mut this, src, line, context) {
                        return Poll::Ready(Some(val));
                    }
                }

                Poll::Ready(None) => {
                    // We got `None`, this means the `inner` stream has ended. Start flushing all
                    // existing data, stop polling `inner`.
                    *this.draining = Some(
                        this.logic
                            .buffers
                            .drain()
                            .map(|(src, (_, aggregate))| {
                                let (line, context) = aggregate.merge();
                                (src, line, context)
                            })
                            .collect(),
                    );
                }

                Poll::Pending => {
                    // We didn't get any lines from `inner`, so we just give a line from keys
                    // that have hit their timeout.
                    while let Poll::Ready(Some(Ok(expired_key))) = this.logic.timeouts.poll_expired(cx) {
                        let key = expired_key.into_inner();
                        if let Some((_, aggregate)) = this.logic.buffers.remove(&key) {
                            let (line, context) = aggregate.merge();
                            return Poll::Ready(Some((key, line, context)));
                        }
                    }

                    return Poll::Pending;
                }
            }
        }
    }
}

impl<T, K, C> LineAgg<T, K, C>
    where
        T: Stream<Item=(K, Bytes, C)> + Unpin,
        K: Hash + Eq + Clone,
{
    /// Handle line and do stashing of extra emitted lines.
    /// Requires that the `stashed` item is empty(i.e. entry is vacant). This invariant has
    /// to be taken care of by the caller.
    fn handle_line_and_stashing(
        this: &mut LineAggProj<'_, T, K, C>,
        src: K,
        line: Bytes,
        context: C,
    ) -> Option<(K, Bytes, C)> {
        // Stashed line is always consumed at the start of the `poll` loop before entering
        // this line processing logic. If it's non-empty here - it's a bug.
        debug_assert!(this.stashed.is_none());
        let val = this.logic.handle_line(src, line, context)?;
        let val = match val {
            // If we have to emit just one line - that's easy, we just return it.
            (src, Emit::One((line, context))) => (src, line, context),
            // If we have to emit two lines - take the second one and stash it, then return
            // the first one. This way, the stashed line will be returned on the next
            // stream poll
            (src, Emit::Two((line, context), (line_to_stash, context_to_stash))) => {
                *this.stashed = Some((src.clone(), line_to_stash, context_to_stash));
                (src, line, context)
            }
        };

        Some(val)
    }
}