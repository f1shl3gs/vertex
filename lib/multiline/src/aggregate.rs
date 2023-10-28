use std::collections::{hash_map::Entry, HashMap};
use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use futures::Stream;
use futures::StreamExt;
use pin_project_lite::pin_project;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio_util::time::delay_queue::Key;
use tokio_util::time::DelayQueue;

use super::serde_regex;

/// The mode of operation of the line aggregator
#[allow(clippy::derive_partial_eq_without_eq)]
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

pin_project! {
    /// Line aggregator.
    ///
    /// Provides a `Stream` implementation that reads lines from the `inner` stream
    /// and yields aggregated lines.
    #[project = LineAggProj]
    pub struct LineAgg<T, R, K, C> {
        // The stream from which we read the lines.
        #[pin]
        inner: T,

        // The core line aggregation logic.
        logic: Logic<R, K, C>,

        // Stashed lines. When line aggregation results in more than one line being emitted,
        // we have to stash lines and return them into the stream after that before doing any
        // other work
        stashed: Option<(K, Bytes, C)>,

        // Duration queue. We switch to draining mode when we get `None` from the inner stream.
        // In this mode we stop polling `inner` for new lines and just flush all the buffered data.
        draining: Option<Vec<(K, Bytes, C)>>,
    }
}

/// Rule is extract from core logic, so we can implement preset easily and, implement it
/// as we wish, the performance of regex is not good, so we can implement by something like
/// `contains`, it should be blazing fast.
pub trait Rule {
    fn is_start(&self, line: &Bytes) -> bool;
    fn is_condition(&self, line: &Bytes) -> bool;
    fn mode(&self) -> Mode;
}

/// Core line aggregation logic
///
/// Encapsulates the essential state and the core logic for the line aggregation algorithm
pub struct Logic<R, K, C> {
    /// Configuration parameters to use.
    rule: R,

    /// Timeout for multiline aggregate
    timeout: Duration,

    /// Line per key
    /// Key is usually a filename or other line source identifier.
    buffers: HashMap<K, (Key, Aggregate<C>)>,

    /// A queue of key timeouts
    timeouts: DelayQueue<K>,
}

impl<R, K, C> Logic<R, K, C> {
    /// Create a new `Logic` using the specified `Config`
    pub fn new(rule: R, timeout: Duration) -> Self {
        Self {
            rule,
            timeout,
            buffers: HashMap::new(),
            timeouts: DelayQueue::new(),
        }
    }
}

impl<T, R, K, C> LineAgg<T, R, K, C>
where
    T: Stream<Item = (K, Bytes, C)> + Unpin,
    K: Hash + Eq + Clone,
    R: Rule,
{
    /// Create a new `LineAgg` using the specified `inner` stream and preconfigured `logic`
    pub fn new(inner: T, logic: Logic<R, K, C>) -> Self {
        Self {
            inner,
            logic,
            draining: None,
            stashed: None,
        }
    }
}

impl<T, R, K, C> Stream for LineAgg<T, R, K, C>
where
    T: Stream<Item = (K, Bytes, C)> + Unpin,
    K: Hash + Eq + Clone,
    R: Rule,
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
                return match to_drain.pop() {
                    Some(val) => Poll::Ready(Some(val)),
                    _ => Poll::Ready(None),
                };
            }

            match this.inner.poll_next_unpin(cx) {
                Poll::Ready(Some((src, line, context))) => {
                    // Handle the incoming line we got from `inner`. If the handler gave us
                    // something - return it, otherwise continue with the flow.
                    if let Some(val) = Self::handle_line_and_stashing(&mut this, src, line, context)
                    {
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
                    while let Poll::Ready(Some(expired_key)) = this.logic.timeouts.poll_expired(cx)
                    {
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

impl<T, R, K, C> LineAgg<T, R, K, C>
where
    T: Stream<Item = (K, Bytes, C)> + Unpin,
    K: Hash + Eq + Clone,
    R: Rule,
{
    /// Handle line and do stashing of extra emitted lines.
    /// Requires that the `stashed` item is empty(i.e. entry is vacant). This invariant has
    /// to be taken care of by the caller.
    fn handle_line_and_stashing(
        this: &mut LineAggProj<'_, T, R, K, C>,
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

/// Specifies the amount of lines to emit in response to a single input line.
/// We have to emit either one or two lines.
pub enum Emit<T> {
    /// Emit one line.
    One(T),
    /// Emit two lines, in the order they're specified
    Two(T, T),
}

/// A helper enum
enum Decision {
    Continue,
    EndInclude,
    EndExclude,
}

impl<R, K, C> Logic<R, K, C>
where
    K: Hash + Eq + Clone,
    R: Rule,
{
    /// Handle line, if we have something to output - return it.
    pub fn handle_line(
        &mut self,
        src: K,
        line: Bytes,
        context: C,
    ) -> Option<(K, Emit<(Bytes, C)>)> {
        // Check if we already have the buffered data for the source
        match self.buffers.entry(src) {
            Entry::Occupied(mut entry) => {
                let condition_matched = self.rule.is_condition(&line);
                let decision = match (self.rule.mode(), condition_matched) {
                    // All consecutive lines matching this pattern are included in the group
                    (Mode::ContinueThrough, true) => Decision::Continue,
                    (Mode::ContinueThrough, false) => Decision::EndExclude,
                    // All consecutive lines matching this pattern, plus one additional line,
                    // are included in the group
                    (Mode::ContinuePast, true) => Decision::Continue,
                    (Mode::ContinuePast, false) => Decision::EndInclude,
                    // All consecutive lines not matching this pattern are included in the group
                    (Mode::HaltBefore, true) => Decision::EndExclude,
                    (Mode::HaltBefore, false) => Decision::Continue,
                    // All consecutive lines, up to and including the first line matching this
                    // pattern, are included in the group
                    (Mode::HaltWith, true) => Decision::EndInclude,
                    (Mode::HaltWith, false) => Decision::Continue,
                };

                match decision {
                    Decision::Continue => {
                        let buffered = entry.get_mut();
                        self.timeouts.reset(&buffered.0, self.timeout);
                        buffered.1.add_next_line(line);
                        None
                    }
                    Decision::EndInclude => {
                        let (src, (key, mut buffered)) = entry.remove_entry();
                        self.timeouts.remove(&key);
                        buffered.add_next_line(line);
                        Some((src, Emit::One(buffered.merge())))
                    }
                    Decision::EndExclude => {
                        let (src, (key, buffered)) = entry.remove_entry();
                        self.timeouts.remove(&key);
                        Some((src, Emit::Two(buffered.merge(), (line, context))))
                    }
                }
            }
            Entry::Vacant(entry) => {
                // This line is a candidate for buffering, or passing through
                if self.rule.is_start(&line) {
                    // It was indeed a new line we need to filter. Set the timeout
                    // buffer this line.
                    let key = self.timeouts.insert(entry.key().clone(), self.timeout);
                    entry.insert((key, Aggregate::new(line, context)));
                    None
                } else {
                    // It's just a regular line we don't really care about
                    Some((entry.into_key(), Emit::One((line, context))))
                }
            }
        }
    }
}

struct Aggregate<C> {
    lines: Vec<Bytes>,
    context: C,
}

impl<C> Aggregate<C> {
    fn new(first_line: Bytes, context: C) -> Self {
        Self {
            lines: vec![first_line],
            context,
        }
    }

    fn add_next_line(&mut self, line: Bytes) {
        self.lines.push(line)
    }

    fn merge(self) -> (Bytes, C) {
        let capacity = self.lines.iter().map(|line| line.len() + 1).sum::<usize>() - 1;
        let mut bytes_mut = BytesMut::with_capacity(capacity);
        let mut first = true;
        for line in self.lines {
            if first {
                first = false
            } else {
                bytes_mut.extend_from_slice(b"\n");
            }

            bytes_mut.extend_from_slice(&line);
        }

        (bytes_mut.freeze(), self.context)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegexRule {
    #[serde(with = "serde_regex")]
    pub start_pattern: regex::bytes::Regex,
    #[serde(with = "serde_regex")]
    pub condition_pattern: regex::bytes::Regex,

    pub mode: Mode,
}

impl Rule for RegexRule {
    fn is_start(&self, line: &Bytes) -> bool {
        self.start_pattern.is_match(line.as_ref())
    }

    fn is_condition(&self, line: &Bytes) -> bool {
        self.condition_pattern.is_match(line.as_ref())
    }

    fn mode(&self) -> Mode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    type Filename = String;

    fn stream_from_lines<'a>(
        lines: &'a [&'static str],
    ) -> impl Stream<Item = (Filename, Bytes, ())> + 'a {
        futures::stream::iter(lines.iter().map(|line| {
            (
                "test.log".to_owned(),
                Bytes::from_static(line.as_bytes()),
                (),
            )
        }))
    }

    fn assert_results(actual: Vec<(Filename, Bytes, ())>, expected: &[&str]) {
        let expected_mapped: Vec<(Filename, Bytes, ())> = expected
            .iter()
            .map(|line| {
                (
                    "test.log".to_owned(),
                    Bytes::copy_from_slice(line.as_bytes()),
                    (),
                )
            })
            .collect();

        assert_eq!(
            actual, expected_mapped,
            "actual on the left, expected on the right"
        )
    }

    async fn run_and_assert(lines: &[&'static str], rule: impl Rule, expected: &[&'static str]) {
        let stream = stream_from_lines(lines);
        let logic = Logic::new(rule, Duration::from_secs(5));
        let aggr = LineAgg::new(stream, logic);
        let results = aggr.collect().await;
        assert_results(results, expected)
    }

    #[tokio::test]
    async fn mode_continue_through_1() {
        let lines = vec![
            "some usual line",
            "some other usual line",
            "first part",
            " second part",
            " last part",
            "another normal message",
            "finishing message",
            " last part of the incomplete finishing message",
        ];

        let rule = RegexRule {
            start_pattern: regex::bytes::Regex::new("^[^\\s]").unwrap(),
            condition_pattern: regex::bytes::Regex::new("^[\\s]+").unwrap(),
            mode: Mode::ContinueThrough,
        };

        let expected = vec![
            "some usual line",
            "some other usual line",
            concat!("first part\n", " second part\n", " last part"),
            "another normal message",
            concat!(
                "finishing message\n",
                " last part of the incomplete finishing message"
            ),
        ];

        run_and_assert(&lines, rule, &expected).await;
    }
}
