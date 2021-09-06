/// Exposes available entropy

use crate::{
    tags,
    gauge_metric,
    event::{Metric, MetricValue},
    sources::node::errors::Error,
    sources::node::read_into,
};

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, ()> {
    let (avail, pool_size) = read_random(proc_path).await.map_err(|err| {
        warn!("read random stat failed, {}", err);
    })?;

    Ok(vec![
        gauge_metric!(
            "node_entropy_available_bits",
            "Bits of available entropy.",
            avail as f64,
        ),
        gauge_metric!(
            "node_entropy_pool_size_bits",
            "Bits of entropy pool.",
            pool_size as f64
        ),
    ])
}

async fn read_random(proc_path: &str) -> Result<(u64, u64), Error> {
    let path = format!("{}/sys/kernel/random/entropy_avail", proc_path);
    let avail = read_into(path).await?;

    let path = format!("{}/sys/kernel/random/poolsize", proc_path);
    let pool_size = read_into(path).await?;

    Ok((avail, pool_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_random() {
        let path = "testdata/proc";
        let (avail, pool_size) = read_random(path).await.unwrap();

        assert_eq!(avail, 3943);
        assert_eq!(pool_size, 4096);
    }
}