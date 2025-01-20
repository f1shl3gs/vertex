use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;

pub async fn wait_for_tcp(addr: SocketAddr) {
    let start = Instant::now();
    let timeout = Duration::from_secs(60);

    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                if start.elapsed() > timeout {
                    panic!("Timed out waiting for connection");
                }
            },
            result = TcpStream::connect(&addr) => {
                match result {
                    Ok(_conn) => break,
                    Err(_err) => {}
                }

                if start.elapsed() > timeout {
                    panic!("Timed out waiting for connection");
                }
            }
        }
    }
}
