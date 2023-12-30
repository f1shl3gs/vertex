#![allow(clippy::print_stdout)]

use ntp::Client;

#[tokio::main]
async fn main() {
    let client = Client::default();
    let resp = client.query("ntp.aliyun.com:123").await.unwrap();

    // Display of std::time::Duration is more pretty and human-readable
    println!("precision:        {:?}", resp.precision.to_std().unwrap());
    println!(
        "clock offset:     {:?}",
        resp.clock_offset.to_std().unwrap()
    );
    println!("root delay:       {:?}", resp.root_delay.to_std().unwrap());
    println!(
        "root dispersion:  {:?}",
        resp.root_dispersion.to_std().unwrap()
    );
    println!(
        "root distance:    {:?}",
        resp.root_distance.to_std().unwrap()
    );
}
