mod driver;
mod partitioned_batcher;
mod futures_unordered_chunked;
mod concurrent_map;
mod timer;
pub mod batcher;

pub use partitioned_batcher::*;
pub use driver::*;
pub use concurrent_map::ConcurrentMap;


pub async fn tripwire_handler(closed: bool) {
    futures::future::poll_fn(|_| {
        if closed {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    }).await
}