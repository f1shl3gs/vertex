use crate::{Error, FileOpenFailed, OtherErr, ProcFS};
use snafu::ResultExt;
use std::collections::BTreeMap;
use tokio::io::AsyncBufReadExt;

impl ProcFS {
    pub async fn arp_entries(&self) -> Result<BTreeMap<String, u64>, Error> {
        let path = &self.root.join("net/arp");
        let f = tokio::fs::File::open(path)
            .await
            .context(FileOpenFailed { path })?;
        let reader = tokio::io::BufReader::new(f);
        let mut lines = reader.lines();
        let mut devices = BTreeMap::new();

        // skip the first line
        lines.next_line().await.context(OtherErr)?;

        while let Some(line) = lines.next_line().await.context(OtherErr)? {
            let dev = line.split_ascii_whitespace().nth(5).unwrap();

            match devices.get_mut(dev) {
                Some(v) => *v += 1u64,
                _ => {
                    devices.insert(dev.into(), 1u64);
                }
            }
        }

        Ok(devices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_arp_entries() {
        let cd = std::env::current_dir().unwrap();
        println!("{:?}", cd);

        let procfs = ProcFS::test_procfs();
        let entries = procfs.arp_entries().await.unwrap();
    }
}
