use std::collections::BTreeMap;

use snafu::ResultExt;
use tokio::io::AsyncBufReadExt;

use crate::{Error, FileOpenFailedSnafu, OtherErrSnafu, ProcFS};

impl ProcFS {
    pub async fn arp_entries(&self) -> Result<BTreeMap<String, u64>, Error> {
        let path = &self.root.join("net/arp");
        let f = tokio::fs::File::open(path)
            .await
            .context(FileOpenFailedSnafu { path })?;
        let reader = tokio::io::BufReader::new(f);
        let mut lines = reader.lines();
        let mut devices = BTreeMap::new();

        // skip the first line
        lines.next_line().await.context(OtherErrSnafu)?;

        while let Some(line) = lines.next_line().await.context(OtherErrSnafu)? {
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
        let procfs = ProcFS::test_procfs();
        let _entries = procfs.arp_entries().await.unwrap();
    }
}
