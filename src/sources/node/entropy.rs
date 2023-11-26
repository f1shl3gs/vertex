//! Exposes available entropy

use std::path::PathBuf;

use event::Metric;

use super::{read_into, Error};

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
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

async fn read_random(proc_path: PathBuf) -> Result<(u64, u64), Error> {
    let avail = read_into(proc_path.join("sys/kernel/random/entropy_avail"))?;
    let pool_size = read_into(proc_path.join("sys/kernel/random/poolsize"))?;

    Ok((avail, pool_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_random() {
        let path = "tests/fixtures/proc".into();
        let (avail, pool_size) = read_random(path).await.unwrap();

        assert_eq!(avail, 3943);
        assert_eq!(pool_size, 4096);
    }
}
