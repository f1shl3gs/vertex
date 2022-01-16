use crate::{Error, ProcFS};
use tokio::io::AsyncBufReadExt;

// A ConntrackStatEntry represents one line from net/stat/nf_conntrack
// and contains netfilter conntrack statistics at one CPU core
#[derive(Debug, Default)]
pub struct ConntrackStatEntry {
    pub entries: u64,
    pub found: u64,
    pub invalid: u64,
    pub ignore: u64,
    pub insert: u64,
    pub insert_failed: u64,
    pub drop: u64,
    pub early_drop: u64,
    pub search_restart: u64,
}

impl ConntrackStatEntry {
    fn new(line: &str) -> Result<Self, Error> {
        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
        if parts.len() != 17 {
            return Err(Error::invalid_data("no processor found"));
        }

        let entries = hex_u64(parts[0].as_bytes())?;
        let found = hex_u64(parts[2].as_bytes())?;
        let invalid = hex_u64(parts[4].as_bytes())?;
        let ignore = hex_u64(parts[5].as_bytes())?;
        let insert = hex_u64(parts[8].as_bytes())?;
        let insert_failed = hex_u64(parts[9].as_bytes())?;
        let drop = hex_u64(parts[10].as_bytes())?;
        let early_drop = hex_u64(parts[11].as_bytes())?;
        let search_restart = hex_u64(parts[16].as_bytes())?;

        Ok(Self {
            entries,
            found,
            invalid,
            ignore,
            insert,
            insert_failed,
            drop,
            early_drop,
            search_restart,
        })
    }
}

impl ProcFS {
    pub async fn conntrack(&self) -> Result<Vec<ConntrackStatEntry>, Error> {
        let path = self.root.join("net/stat/nf_conntrack");
        let f = tokio::fs::File::open(path).await?;
        let r = tokio::io::BufReader::new(f);
        let mut lines = r.lines();

        let mut first = true;
        let mut stats = Vec::new();

        while let Some(line) = lines.next_line().await? {
            if first {
                first = false;
                continue;
            }

            if let Ok(ent) = ConntrackStatEntry::new(&line) {
                stats.push(ent);
            }
        }

        Ok(stats)
    }
}

#[inline]
fn hex_u64(input: &[u8]) -> Result<u64, Error> {
    let res = input
        .iter()
        .rev()
        .enumerate()
        .map(|(k, &v)| {
            let digit = v as char;
            (digit.to_digit(16).unwrap_or(0) as u64) << (k * 4)
        })
        .sum();

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conntrack_new() {
        let line = "0000000a  00000000 00000000 00000000 00000005 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000  00000000 00000000 00000000 00000004";
        let ent = ConntrackStatEntry::new(line).unwrap();

        assert_eq!(ent.search_restart, 4)
    }

    #[test]
    fn test_hex_u64() {
        let v = hex_u64(b"0000000a").unwrap();
        assert_eq!(v, 10u64)
    }
}
