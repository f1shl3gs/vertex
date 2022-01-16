use tokio::io::AsyncBufReadExt;

use crate::{Error, ProcFS};

pub const USER_HZ: f64 = 100.0;

#[derive(Default)]
pub struct CPUStat {
    pub user: f64,
    pub nice: f64,
    pub system: f64,
    pub idle: f64,
    pub iowait: f64,
    pub irq: f64,
    pub softirq: f64,
    pub steal: f64,
    pub guest: f64,
    pub guest_nice: f64,
}

impl ProcFS {
    pub async fn cpu(&self) -> Result<Vec<CPUStat>, Error> {
        let path = self.root.join("stat");

        let f = tokio::fs::File::open(path).await?;
        let reader = tokio::io::BufReader::new(f);
        let mut lines = reader.lines();
        let mut stats = Vec::new();

        while let Some(line) = lines.next_line().await? {
            if !line.starts_with("cpu") {
                continue;
            }

            if line.starts_with("cpu ") {
                continue;
            }

            let parts = line.split_ascii_whitespace();
            let mut stat = CPUStat::default();

            for (index, part) in parts.enumerate().skip(1) {
                let v = part.parse().unwrap_or(0f64) / USER_HZ;

                match index {
                    1 => stat.user = v,
                    2 => stat.nice = v,
                    3 => stat.system = v,
                    4 => stat.idle = v,
                    5 => stat.iowait = v,
                    6 => stat.irq = v,
                    7 => stat.softirq = v,
                    8 => stat.steal = v,
                    9 => stat.guest = v,
                    10 => stat.guest_nice = v,
                    _ => unreachable!(),
                }
            }

            stats.push(stat);
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_cpu_stats() {
        let procfs = ProcFS::test_procfs();
        let stats = procfs.cpu().await.unwrap();

        assert_eq!(stats.len(), 8);
        assert_eq!(31f64 / USER_HZ, stats[7].softirq);
        assert_eq!(1f64 / USER_HZ, stats[0].irq);
        assert_eq!(47869f64 / USER_HZ, stats[1].user);
        assert_eq!(23f64 / USER_HZ, stats[1].nice);
        assert_eq!(15916f64 / USER_HZ, stats[2].system);
        assert_eq!(1113230f64 / USER_HZ, stats[3].idle);
        assert_eq!(217f64 / USER_HZ, stats[4].iowait);
        assert_eq!(0f64 / USER_HZ, stats[5].irq);
        assert_eq!(29f64 / USER_HZ, stats[6].softirq);
        assert_eq!(0f64, stats[7].steal);
        assert_eq!(0f64, stats[7].guest);
        assert_eq!(0f64, stats[7].guest_nice);
    }
}
