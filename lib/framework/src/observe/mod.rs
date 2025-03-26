mod endpoint;
mod observer;

pub use endpoint::Endpoint;
pub(crate) use observer::receiver_count;
pub use observer::{Change, Notifier, Observer, available_observers, current_endpoints, subscribe};

/// `run` is a simple helper for period service discovery, others with WATCH mechanism is
/// not suitable for this function.
pub async fn run<L>(
    observer: Observer,
    interval: std::time::Duration,
    mut shutdown: crate::ShutdownSignal,
    mut list_endpoints: L,
) -> Result<(), ()>
where
    L: AsyncFnMut() -> crate::Result<Vec<Endpoint>>,
{
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        match list_endpoints().await {
            Ok(endpoints) => {
                if let Err(_err) = observer.publish(endpoints) {
                    warn!(message = "publish endpoints failed");
                    break;
                }
            }
            Err(err) => {
                warn!(message = "error while listing endpoints", ?err);
            }
        }
    }

    Ok(())
}
