mod client;
mod error;
mod request;

pub use client::Client;
pub use error::Error;

#[cfg(test)]
mod tests {
    use crate::Client;

    #[tokio::test]
    #[ignore]
    #[allow(clippy::print_stdout)]
    async fn get_version() {
        let mut cli = Client::connect("/run/libvirt/libvirt-sock-ro")
            .await
            .unwrap();
        cli.open().await.unwrap();
        let ver = cli.version().await.unwrap();
        println!("version: {}", ver);
    }
}
