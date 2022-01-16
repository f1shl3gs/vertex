use crate::{read_into, Error, ProcFS};

pub struct Entropy {
    pub avail: u64,
    pub pool_size: u64,
}

impl ProcFS {
    pub async fn entryopy(&self) -> Result<Entropy, Error> {
        let path = self.root.join("sys/kernel/random/entropy_avail");
        let avail = read_into(path).await?;
        let path = self.root.join("sys/kernel/random/poolsize");
        let pool_size = read_into(path).await?;

        Ok(Entropy { avail, pool_size })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_random() {
        let procfs = ProcFS::test_procfs();
        let entropy = procfs.entryopy().await.unwrap();

        assert_eq!(entropy.avail, 3943);
        assert_eq!(entropy.pool_size, 4096);
    }
}
