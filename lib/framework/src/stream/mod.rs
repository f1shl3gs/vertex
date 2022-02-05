pub mod batcher;
mod concurrent_map;
mod driver;
mod futures_unordered_chunked;
mod partitioned_batcher;
mod timer;

pub use concurrent_map::ConcurrentMap;
pub use driver::*;
pub use partitioned_batcher::*;

pub async fn tripwire_handler(closed: bool) {
    futures::future::poll_fn(|_| {
        if closed {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    })
    .await
}
