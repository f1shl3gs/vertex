use std::task::{Context, Poll};

/// A trait for representing a timer which holds multiple subtimers, mapped by an arbitrary key, `K`.
///
/// Embedding time as a type into other types eases property testing and verification. As such, this
/// trait represents the minimum functionality required to describe management of keyed timers for
/// the types implemented in this crate that require such behaviour.
///
/// Users can look at `ExpirationQueue` for a concrete implement
pub trait KeyedTimer<K> {
    /// Clear the timer.
    ///
    /// Clears all keys from the timer. Futures calls to `poll_expired` will return `None` until
    /// another key is added
    fn clear(&mut self);

    /// Insert a new subtimer, keyedby `K`.
    ///
    /// If the given key already exists in the timer, the underlying subtimer is reset.
    fn insert(&mut self, key: K);

    /// Attempts to pull out the next expired subtimer in the queue.
    ///
    /// The key of the subtimer is returned if it has expired, otherwise, returns `None` if the
    /// queue is exhausted.
    ///
    /// Unlike a typical stream, returning `None` only indicates that the queue is empty, not
    /// that the queue will never return anything else in the future.
    ///
    /// Used primarily for property testing vis-a-vis `Batcher`
    fn poll_expired(&mut self, cx: &mut Context) -> Poll<Option<K>>;
}
