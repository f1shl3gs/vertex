use crate::event::Metric;
use std::os::unix::fs::MetadataExt;

pub async fn gather(sys_path: &str) -> Result<Vec<Metric>, ()> {
    let path = format!("{}/class/hwmon", sys_path);
    let mut dirs = tokio::fs::read_dir(path).await
        .map_err(|err| {
            warn!("read hwmon dir failed"; "err" => err);
        })?;

    while let Some(entry) = dirs.next_entry().await.map_err(|err| {
        warn!("read next entry of hwmon dirs failed"; "err" => err);
    })? {
        println!("{:?}", entry);

        let meta = entry.metadata().await.map_err(|err| {
            warn!("read metadata failed"; "err" => err);
        })?;

        if !meta.is_dir() {
            continue;
        }

        let file_type = meta.file_type();
        if file_type.is_symlink() {
            continue;
        }

        println!("handle {:?}", entry);
    }

    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gather() {
        let path = "testdata/sys";

        gather(path).await.unwrap();
    }
}
