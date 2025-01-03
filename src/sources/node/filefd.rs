use std::path::PathBuf;

use event::Metric;

use super::{read_string, Error};

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let (allocated, maximum) = read_file_nr(proc_path).await?;

    Ok(vec![
        Metric::gauge(
            "node_filefd_allocated",
            "File descriptor statistics: allocated",
            allocated,
        ),
        Metric::gauge(
            "node_filefd_maximum",
            "File descriptor statistics: maximum",
            maximum,
        ),
    ])
}

async fn read_file_nr(proc_path: PathBuf) -> Result<(u64, u64), Error> {
    let content = read_string(proc_path.join("sys/fs/file-nr"))?;

    // the file-nr proc is only 1 line with 3 values
    let parts = content.split_ascii_whitespace().collect::<Vec<_>>();

    let allocated = parts[0].parse()?;
    let maximum = parts[2].parse()?;

    Ok((allocated, maximum))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_file_nr() {
        let path = "tests/node/proc".into();
        let (allocated, maximum) = read_file_nr(path).await.unwrap();

        assert_eq!(allocated, 1024);
        assert_eq!(maximum, 1631329);
    }
}
