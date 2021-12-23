use bytes::{Bytes, BytesMut};
use futures::Stream;
use futures_util::StreamExt;
use pin_project::pin_project;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time;
use tokio_util::time::delay_queue::Key;
use tokio_util::time::DelayQueue;

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

pub trait Handler<K, C> {
    /// Handle line, if we have something to output - return it.
    fn handle_line(&mut self, src: K, line: Bytes, ctx: C) -> Option<(K, Emit<(Bytes, C)>)>;

    fn is_start(&self, line: &Bytes) -> bool {
        true
    }
}

pub struct Logic<K, C, H> {
    handler: H,

    buffers: HashMap<K, (Key, Aggregate<C>)>,

    delay_queue: DelayQueue<K>,

    timeout: time::Duration,
}

impl<K, C, H> Logic<K, C, H>
where
    H: Handler<K, C>,
{
    pub fn new(handler: H) -> Self {
        Self {
            handler,
            timeout: time::Duration::from_secs(5),
            buffers: HashMap::new(),
            delay_queue: DelayQueue::new(),
        }
    }

    pub fn handle_line(&mut self, src: K, line: Bytes, ctx: C) -> Option<(K, Emit<(Bytes, C)>)>
    where
        K: Hash + Eq + Clone,
    {
        // Check if we already have the buffered data for the source
        match self.buffers.entry(src) {
            Entry::Occupied(mut entry) => {
                if self.handler.is_start(&line) {
                    let (src, (key, mut buffered)) = entry.remove_entry();
                    Some((src, Emit::One((line, ctx))))
                } else {
                    let mut buffered = entry.get_mut();
                    self.delay_queue.reset(&buffered.0, self.timeout);
                    None
                }
            }
            Entry::Vacant(entry) => {
                // This line is a candidate for buffering, or passing through
                if self.handler.is_start(&line) {
                    let key = self.delay_queue.insert(entry.key().clone(), self.timeout);
                    entry.insert((key, Aggregate::new(line, ctx)));
                    None
                } else {
                    // It's just a regular line we don't really care about
                    Some((entry.into_key(), Emit::One((line, ctx))))
                }
            }
        }
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

#[pin_project(project = LineAggrProj)]
pub struct LineAggr<T, K, C, H> {
    #[pin]
    inner: T,

    logic: Logic<K, C, H>,

    stashed: Option<(K, Bytes, C)>,

    draining: Option<Vec<(K, Bytes, C)>>,
}

impl<T, K, C, H> LineAggr<T, K, C, H>
where
    T: Stream<Item = (K, Bytes, C)> + Unpin,
    K: Hash + Eq + Clone,
{
    pub fn new(inner: T, logic: Logic<K, C, H>) -> Self {
        Self {
            inner,
            logic,
            draining: None,
            stashed: None,
        }
    }
}

impl<T, K, C, H> Stream for LineAggr<T, K, C, H>
where
    T: Stream<Item = (K, Bytes, C)> + Unpin,
    K: Hash + Eq + Clone,
    H: Handler<K, C>,
{
    type Item = (K, Bytes, C);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            // If we have a stashed line, process it before doing anything else.
            if let Some((src, line, ctx)) = this.stashed.take() {
                // Handle the stashed line. If the handler gave us something - return it,
                // otherwise restart the loop iteration to start anew. Handler could've stashed
                // another value, continuing to the new loop iteration handles that.
                if let Some(val) = Self::handle_line_and_stashing(&mut this, src, line, ctx) {
                    return Poll::Ready(Some(val));
                }

                continue;
            }

            // If we're in draining mode, short circuit here
            if let Some(to_drain) = &mut this.draining {
                return match to_drain.pop() {
                    Some(val) => Poll::Ready(Some(val)),
                    _ => Poll::Ready(None),
                };
            }

            match this.inner.poll_next_unpin(cx) {
                Poll::Ready(Some((src, line, ctx))) => {
                    // Handle the incoming line we got from `inner`. If the handler gave us
                    // something - return it, otherwise continue with the flow
                    if let Some(val) = Self::handle_line_and_stashing(&mut this, src, line, ctx) {
                        return Poll::Ready(Some(val));
                    }
                }

                Poll::Ready(None) => {
                    // We got `None`, this means the `inner` stream has ended. Start flushing
                    // all existing data, stop polling `inner`
                    *this.draining = Some(
                        this.logic
                            .buffers
                            .drain()
                            .map(|(src, (_, agg))| {
                                let (line, ctx) = agg.merge();
                                (src, line, ctx)
                            })
                            .collect(),
                    );
                }

                Poll::Pending => {
                    // We didn't get any lines from `inner`, so we just give a line from keys
                    // that have hit their timeout.
                    while let Poll::Ready(Some(Ok(expired_key))) =
                        this.logic.delay_queue.poll_expired(cx)
                    {
                        let key = expired_key.into_inner();
                        if let Some((_, aggr)) = this.logic.buffers.remove(&key) {
                            let (line, ctx) = aggr.merge();
                            return Poll::Ready(Some((key, line, ctx)));
                        }
                    }

                    return Poll::Pending;
                }
            }
        }
    }
}

impl<T, K, C, H> LineAggr<T, K, C, H>
where
    T: Stream<Item = (K, Bytes, C)> + Unpin,
    K: Hash + Eq + Clone,
    H: Handler<K, C>,
{
    /// Handle line and do stashing of extra emitted lines.
    /// Requires that the `stashed` item is empty(i.e. entry is vacant). This invariant
    /// has to be taken care of by the caller
    fn handle_line_and_stashing(
        this: &mut LineAggrProj<'_, T, K, C, H>,
        src: K,
        line: Bytes,
        ctx: C,
    ) -> Option<(K, Bytes, C)> {
        // Stashed line is always consumed at the start of the `poll` loop before entering
        // this line processing logic. If it's non-empty here - it's a bug.
        debug_assert!(this.stashed.is_none());
        let val = this.logic.handle_line(src, line, ctx)?;
        let val = match val {
            // If we have to emit just one line - that's easy, we just return it.
            (src, Emit::One((line, context))) => (src, line, context),
            // If we have to emit two lines - take the second one and stash it, then return
            // the first one. This way, the stashed line will be returned on the next
            // stream poll
            (src, Emit::Two((line, ctx), (line_to_stash, context_to_stash))) => {
                *this.stashed = Some((src.clone(), line_to_stash, context_to_stash));
                (src, line, ctx)
            }
        };

        Some(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

    /// Private type alias to be more expressive in the internal implementation.
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

    #[tokio::test]
    async fn compile() {
        struct FooHandler {}

        impl<K, C> Handler<K, C> for FooHandler {
            fn handle_line(
                &mut self,
                src: K,
                line: Bytes,
                ctx: C,
            ) -> Option<(K, Emit<(Bytes, C)>)> {
                let s = std::str::from_utf8(line.as_ref()).unwrap();
                if s.contains("foo") {
                    Some((src, Emit::One((line, ctx))))
                } else {
                    None
                }
            }
        }

        let lines = vec![
            "abc",
            " def",
            "  ghi",
            " foo -- 1",
            "foo -- 2",
            "  bar",
            "foo -- 3",
            "bar",
            "bar",
        ];

        let h = FooHandler {};
        let logic = Logic::new(h);
        let stream = stream_from_lines(&lines);
        let line_agg = LineAggr::new(stream, logic);
        let result = line_agg.collect::<Vec<_>>().await;

        result.iter().for_each(|line| {
            println!("{:?}", line);
        });
    }

    // async fn run_and_assert(lines: &['static str], h: impl Handler, expected: )
}
