//! Exposes error detection and correction statistics

use std::path::PathBuf;

use event::tags::Key;
use event::{tags, Metric};

use super::{read_into, Error};

const CONTROLLER_KEY: Key = Key::from_static("controller");

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let paths = glob::glob(&format!(
        "{}/devices/system/edac/mc/mc[0-9]*",
        sys_path.to_string_lossy()
    ))?;

    let mut metrics = Vec::new();
    for path in paths.flatten() {
        let controller = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .strip_prefix("mc")
            .unwrap()
            .to_string();

        let (ce_count, ce_noinfo_count, ue_count, ue_noinfo_count) =
            read_edac_stats(path.clone()).await?;
        metrics.extend([
            Metric::sum_with_tags(
                "node_edac_correctable_errors_total",
                "Total correctable memory errors.",
                ce_count as f64,
                tags!(
                    CONTROLLER_KEY => controller.to_string()
                ),
            ),
            Metric::sum_with_tags(
                "node_edac_uncorrectable_errors_total",
                "Total uncorrectable memory errors.",
                ue_count as f64,
                tags!(
                    CONTROLLER_KEY => controller.to_string()
                ),
            ),
            Metric::sum_with_tags(
                "node_edac_csrow_correctable_errors_total",
                "Total correctable memory errors for this csrow.",
                ce_noinfo_count as f64,
                tags!(
                    CONTROLLER_KEY => controller.to_string(),
                    "csrow" => "unknown",
                ),
            ),
            Metric::sum_with_tags(
                "node_edac_csrow_uncorrectable_errors_total",
                "Total uncorrectable memory errors for this csrow.",
                ue_noinfo_count as f64,
                tags!(
                    CONTROLLER_KEY => controller.to_string(),
                    "csrow" => "unknown",
                ),
            ),
        ]);

        // for each controller, walk the csrow directories
        let csrows = glob::glob(&format!("{}/csrow[0-9]*", path.to_string_lossy()))?;
        for path in csrows.flatten() {
            // looks horrible
            let num = path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .strip_prefix("csrow")
                .unwrap()
                .to_string();

            if let Ok((ce_count, ue_count)) = read_edac_csrow_stats(path).await {
                metrics.extend([
                    Metric::sum_with_tags(
                        "node_edac_csrow_correctable_errors_total",
                        "Total correctable memory errors for this csrow.",
                        ce_count as f64,
                        tags!(
                            CONTROLLER_KEY => controller.to_string(),
                            "csrow" => num.to_string(),
                        ),
                    ),
                    Metric::sum_with_tags(
                        "node_edac_csrow_uncorrectable_errors_total",
                        "Total uncorrectable memory errors for this csrow.",
                        ue_count as f64,
                        tags!(
                            CONTROLLER_KEY => controller.to_string(),
                            "csrow" => num,
                        ),
                    ),
                ]);
            }
        }
    }

    Ok(metrics)
}

async fn read_edac_stats(path: PathBuf) -> Result<(u64, u64, u64, u64), Error> {
    let ce_count = read_into(path.join("ce_count"))?;
    let ce_noinfo_count = read_into(path.join("ce_noinfo_count"))?;
    let ue_count = read_into(path.join("ue_count"))?;
    let ue_noinfo_count = read_into(path.join("ue_noinfo_count"))?;

    Ok((ce_count, ce_noinfo_count, ue_count, ue_noinfo_count))
}

async fn read_edac_csrow_stats(path: PathBuf) -> Result<(u64, u64), Error> {
    let ce_count = read_into(path.join("ce_count"))?;
    let ue_count = read_into(path.join("ue_count"))?;

    Ok((ce_count, ue_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_edac_stats() {
        let path = "tests/fixtures/sys/devices/system/edac/mc/mc0".into();
        let (ce_count, ce_noinfo_count, ue_count, ue_noinfo_count) =
            read_edac_stats(path).await.unwrap();

        assert_eq!(ce_count, 1);
        assert_eq!(ce_noinfo_count, 2);
        assert_eq!(ue_count, 5);
        assert_eq!(ue_noinfo_count, 6);
    }

    #[tokio::test]
    async fn test_read_edac_csrow_stats() {
        let path = "tests/fixtures/sys/devices/system/edac/mc/mc0/csrow0".into();
        let (ce_count, ue_count) = read_edac_csrow_stats(path).await.unwrap();

        assert_eq!(ce_count, 3);
        assert_eq!(ue_count, 4);
    }
}
