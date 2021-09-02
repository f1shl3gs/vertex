use std::collections::BTreeMap;
use crate::{
    tags,
    gauge_metric,
    event::{
        Metric, MetricValue,
    },
};
use crate::sources::node::read_to_string;
use crate::sources::node::errors::Error;

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, ()> {
    let (allocated, maximum) = read_file_nr(proc_path).await.map_err(|err| {
        warn!("read file-nr failed"; "err" => err);
    })?;

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

    let allocated = parts[0].parse().map_err(Error::from)?;
    let maximum = parts[2].parse().map_err(Error::from)?;

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