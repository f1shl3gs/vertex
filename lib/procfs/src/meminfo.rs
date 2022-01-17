use crate::{read_to_string, Error, ProcFS};
use std::collections::HashMap;

impl ProcFS {
    pub async fn meminfo(&self) -> Result<HashMap<String, f64>, Error> {
        let path = self.root.join("meminfo");

        let mut infos = HashMap::new();
        let content = read_to_string(path).await?;
        let lines = content.lines();

        for line in lines {
            let parts = line.split_ascii_whitespace().collect::<Vec<_>>();

            let mut fv = parts[1]
                .parse::<f64>()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

            let mut key = parts[0]
                .replace(":", "")
                .replace("(", "_")
                .replace(")", "_");

            match parts.len() {
                2 => { /* no unit */ }
                3 => {
                    // with unit, we presume kB
                    fv *= 1024.0;
                    if key.ends_with('_') {
                        key += "byte"
                    } else {
                        key += "_bytes";
                    }
                }
                _ => unreachable!(),
            }

            infos.insert(key, fv);
        }

        Ok(infos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_mem_info() {
        let procfs = ProcFS::test_procfs();
        let infos = procfs.meminfo().await.unwrap();

        assert_eq!(infos.get("MemTotal_bytes").unwrap(), &(15666184.0 * 1024.0));
        assert_eq!(
            infos.get("DirectMap2M_bytes").unwrap(),
            &(16039936.0 * 1024.0)
        );
        assert_eq!(infos.get("HugePages_Total").unwrap(), &0.0);
    }
}
