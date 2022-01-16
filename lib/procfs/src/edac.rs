use crate::{read_into, Error, SysFS};
use std::path::PathBuf;

pub struct CsRow {
    pub num: String,
    pub ce_count: u64,
    pub ue_count: u64,
}

pub struct EdacStats {
    controller: String,

    ce_count: u64,
    ce_noinfo_count: u64,
    ue_count: u64,
    ue_noinfo_count: u64,
    csrows: Vec<CsRow>,
}

impl SysFS {
    pub async fn edac(&self) -> Result<Vec<EdacStats>, Error> {
        let pattern = format!(
            "{}/devices/system/edac/mc/mc[0-9]*",
            self.root.to_string_lossy()
        );
        let paths = glob::glob(&pattern)?;

        let mut stats = vec![];
        for path in paths.filter_map(Result::ok) {
            let controller = path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .strip_prefix("mc")
                .unwrap();

            let (ce_count, ce_noinfo_count, ue_count, ue_noinfo_count) =
                read_edac_stats(&path).await?;

            let mut s = EdacStats {
                controller: controller.to_string(),
                ce_count,
                ce_noinfo_count,
                ue_count,
                ue_noinfo_count,
                csrows: vec![],
            };

            // for each controller, walk the csrow directories
            let csrows = glob::glob(&format!("{}/csrow[0-9]*", path.to_string_lossy()))?;
            for path in csrows.filter_map(Result::ok) {
                // looks horrible
                let num = path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .strip_prefix("csrow")
                    .unwrap();

                let (ce_count, ue_count) = read_edac_csrow_stats(&path).await?;
                s.csrows.push(CsRow {
                    num: num.to_string(),
                    ce_count,
                    ue_count,
                });
            }

            stats.push(s);
        }

        Ok(stats)
    }
}

async fn read_edac_stats(path: &PathBuf) -> Result<(u64, u64, u64, u64), Error> {
    let ce_count = read_into(path.join("ce_count")).await?;
    let ce_noinfo_count = read_into(path.join("ce_noinfo_count")).await?;
    let ue_count = read_into(path.join("ue_count")).await?;
    let ue_noinfo_count = read_into(path.join("ue_noinfo_count")).await?;

    Ok((ce_count, ce_noinfo_count, ue_count, ue_noinfo_count))
}

async fn read_edac_csrow_stats(path: &PathBuf) -> Result<(u64, u64), Error> {
    let ce_count = read_into(path.join("ce_count")).await?;
    let ue_count = read_into(path.join("ue_count")).await?;

    Ok((ce_count, ue_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_edac_stats() {
        let path = "fixtures/sys/devices/system/edac/mc/mc0".into();
        let (ce_count, ce_noinfo_count, ue_count, ue_noinfo_count) =
            read_edac_stats(&path).await.unwrap();

        assert_eq!(ce_count, 1);
        assert_eq!(ce_noinfo_count, 2);
        assert_eq!(ue_count, 5);
        assert_eq!(ue_noinfo_count, 6);
    }

    #[tokio::test]
    async fn test_read_edac_csrow_stats() {
        let path = "fixtures/sys/devices/system/edac/mc/mc0/csrow0".into();
        let (ce_count, ue_count) = read_edac_csrow_stats(&path).await.unwrap();

        assert_eq!(ce_count, 3);
        assert_eq!(ue_count, 4);
    }
}
