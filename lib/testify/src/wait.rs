use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tokio::net::TcpStream;
use tokio::time::sleep;

const WAIT_FOR_SECS: u64 = 5; // The default time to wait in `wait_for`
const WAIT_FOR_MIN_MILLIS: u64 = 5; // The minimum time to pause before retrying
const WAIT_FOR_MAX_MILLIS: u64 = 500; // The maximum time to pause before retrying

// Wait for a Future to resolve, or the duration to elapse(will panic)
pub async fn wait_for_duration<F>(mut f: F, duration: Duration)
where
    F: AsyncFnMut() -> bool + Send + 'static,
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
pub async fn wait_for<F>(f: F)
where
    F: AsyncFnMut() -> bool + Send + 'static,
{
    wait_for_duration(f, Duration::from_secs(WAIT_FOR_SECS)).await
}

// Wait (for 10s) for a TCP socket to be reachable
pub async fn wait_for_tcp(addr: SocketAddr) {
    let timeout = Duration::from_secs(20);
    let start = Instant::now();

    loop {
        if let Ok(Ok(_conn)) =
            tokio::time::timeout(Duration::from_millis(500), TcpStream::connect(addr)).await
        {
            break;
        }

        if start.elapsed() > timeout {
            panic!("Timed out waiting for connection");
        }
    }
}
