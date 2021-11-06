/// Exposes available entropy

use super::{Error, ErrorContext, read_into};
use event::{tags, gauge_metric, Metric};

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
    let (avail, pool_size) = read_random(proc_path).await
        .context("read random stat failed")?;

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
        let path = "tests/fixtures/proc";
        let (avail, pool_size) = read_random(path).await.unwrap();

        assert_eq!(avail, 3943);
        assert_eq!(pool_size, 4096);
    }
}