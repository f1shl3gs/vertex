//! Exposes available entropy

use std::path::Path;

use event::Metric;

use super::{Error, Paths, read_into};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let (avail, pool_size) = read_random(paths.proc())?;

    Ok(vec![
        Metric::gauge(
            "node_entropy_available_bits",
            "Bits of available entropy.",
            avail,
        ),
        Metric::gauge(
            "node_entropy_pool_size_bits",
            "Bits of entropy pool.",
            pool_size,
        ),
    ])
}

fn read_random(proc_path: &Path) -> Result<(u64, u64), Error> {
    let avail = read_into(proc_path.join("sys/kernel/random/entropy_avail"))?;
    let pool_size = read_into(proc_path.join("sys/kernel/random/poolsize"))?;

    Ok((avail, pool_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read() {
        let path = Path::new("tests/node/fixtures/proc");
        let (avail, pool_size) = read_random(path).unwrap();

        assert_eq!(avail, 3943);
        assert_eq!(pool_size, 4096);
    }
}
