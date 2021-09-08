use crate::{
    gauge_metric,
    event::{
        Metric, MetricValue,
    },
    sources::node::{
        read_to_string,
        errors::Error,
    },
};
use crate::sources::node::errors::ErrContext;

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
    let (allocated, maximum) = read_file_nr(proc_path).await
        .message("read file-nr failed")?;

    Ok(vec![
        gauge_metric!(
            "node_filefd_allocated",
            "File descriptor statistics: allocated",
            allocated as f64
        ),
        gauge_metric!(
            "node_filefd_maximum",
            "File descriptor statistics: maximum",
            maximum as f64
        ),
    ])
}

async fn read_file_nr(proc_path: &str) -> Result<(u64, u64), Error> {
    let path = format!("{}/sys/fs/file-nr", proc_path);
    let content = read_to_string(path).await?;

    // the file-nr proc is only 1 line with 3 values
    let parts = content.split_ascii_whitespace()
        .collect::<Vec<_>>();

    let allocated = parts[0].parse()?;
    let maximum = parts[2].parse()?;

    Ok((allocated, maximum))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_file_nr() {
        let path = "testdata/proc";
        let (allocated, maximum) = read_file_nr(path).await.unwrap();

        assert_eq!(allocated, 1024);
        assert_eq!(maximum, 1631329);
    }
}