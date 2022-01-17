use crate::{read_to_string, Error, ProcFS};

impl ProcFS {
    pub async fn loadavg(&self) -> Result<(f64, f64, f64), Error> {
        let path = self.root.join("loadavg");
        let content = read_to_string(path).await?;
        let loads = content
            .split_ascii_whitespace()
            .map(|part| part.parse::<f64>().unwrap_or(0.0))
            .collect::<Vec<f64>>();

        Ok((loads[0], loads[1], loads[2]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_loadavg() {
        let procfs = ProcFS::test_procfs();
        let (l1, l5, l15) = procfs.loadavg().await.unwrap();

        assert_eq!(l1, 0.02);
        assert_eq!(l5, 0.04);
        assert_eq!(l15, 0.05);
    }
}
