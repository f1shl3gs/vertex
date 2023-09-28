//! Exposes available entropy

use event::Metric;

use super::{read_into, Error};

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
    let (avail, pool_size) = read_random(proc_path).await?;

    Ok(vec![
        Metric::gauge(
            "node_entropy_available_bits",
            "Bits of available entropy.",
            avail as f64,
        ),
        Metric::gauge(
            "node_entropy_pool_size_bits",
            "Bits of entropy pool.",
            pool_size as f64,
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
