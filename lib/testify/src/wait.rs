use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tokio::net::TcpStream;
use tokio::time::sleep;

const WAIT_FOR_SECS: u64 = 5; // The default time to wait in `wait_for`
const WAIT_FOR_MIN_MILLIS: u64 = 5; // The minimum time to pause before retrying
const WAIT_FOR_MAX_MILLIS: u64 = 500; // The maximum time to pause before retrying

// Wait for a Future to resolve, or the duration to elapse(will panic)
pub async fn wait_for_duration<F, Fut>(mut f: F, duration: Duration)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool> + Send + 'static,
{
    let started = Instant::now();
    let mut delay = WAIT_FOR_MIN_MILLIS;

    while !f().await {
        sleep(Duration::from_millis(delay)).await;

        if started.elapsed() > duration {
            panic!("Timed out while waiting");
        }

        // quadratic backoff up to a maximum delay
        delay = (2 * delay).min(WAIT_FOR_MAX_MILLIS);
    }
}

// Wait for 5s
pub async fn wait_for<F, Fut>(f: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool> + Send + 'static,
{
    wait_for_duration(f, Duration::from_secs(WAIT_FOR_SECS)).await
}

// Wait (for 5s) for a TCP socket to be reachable
pub async fn wait_for_tcp(addr: SocketAddr) {
    wait_for(|| async move { TcpStream::connect(addr).await.is_ok() }).await
}
