use crate::{read_to_string, Error, ProcFS};

impl ProcFS {
    pub async fn filefd(&self) -> Result<(u64, u64), Error> {
        let path = self.root.join("sys/fs/file-nr");

        let content = read_to_string(path).await?;
        // the file-nr proc is only 1 line with 3 values
        let parts = content.split_ascii_whitespace().collect::<Vec<_>>();

        let allocated = parts[0].parse()?;
        let maximum = parts[2].parse()?;

        Ok((allocated, maximum))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_file_nr() {
        let procfs = ProcFS::test_procfs();
        let (allocated, maximum) = procfs.filefd().await.unwrap();

        assert_eq!(allocated, 1024);
        assert_eq!(maximum, 1631329);
    }
}
