//! Exposes error detection and correction statistics

use std::path::{Path, PathBuf};

use event::{Metric, tags};

use super::{Error, read_into};

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let paths = glob::glob(&format!(
        "{}/devices/system/edac/mc/mc[0-9]*",
        sys_path.to_string_lossy()
    ))?;

    let mut metrics = Vec::new();
    for path in paths.flatten() {
        let name = path.file_name().unwrap().to_string_lossy();
        let controller = name.strip_prefix("mc").unwrap();

        let (ce_count, ce_noinfo_count, ue_count, ue_noinfo_count) = read_edac_stats(&path)?;
        metrics.extend([
            Metric::sum_with_tags(
                "node_edac_correctable_errors_total",
                "Total correctable memory errors.",
                ce_count,
                tags!(
                    "controller" => controller
                ),
            ),
            Metric::sum_with_tags(
                "node_edac_uncorrectable_errors_total",
                "Total uncorrectable memory errors.",
                ue_count,
                tags!(
                    "controller" => controller
                ),
            ),
            Metric::sum_with_tags(
                "node_edac_csrow_correctable_errors_total",
                "Total correctable memory errors for this csrow.",
                ce_noinfo_count,
                tags!(
                    "controller" => controller,
                    "csrow" => "unknown",
                ),
            ),
            Metric::sum_with_tags(
                "node_edac_csrow_uncorrectable_errors_total",
                "Total uncorrectable memory errors for this csrow.",
                ue_noinfo_count,
                tags!(
                    "controller" => controller,
                    "csrow" => "unknown",
                ),
            ),
        ]);

        // for each controller, walk the csrow directories
        let csrows = glob::glob(&format!("{}/csrow[0-9]*", path.to_string_lossy()))?;
        for path in csrows.flatten() {
            let name = path.file_name().unwrap().to_string_lossy();
            let num = name.strip_prefix("csrow").unwrap();

            if let Ok((ce_count, ue_count)) = read_edac_csrow_stats(&path) {
                metrics.extend([
                    Metric::sum_with_tags(
                        "node_edac_csrow_correctable_errors_total",
                        "Total correctable memory errors for this csrow.",
                        ce_count,
                        tags!(
                            "controller" => controller,
                            "csrow" => num,
                        ),
                    ),
                    Metric::sum_with_tags(
                        "node_edac_csrow_uncorrectable_errors_total",
                        "Total uncorrectable memory errors for this csrow.",
                        ue_count,
                        tags!(
                            "controller" => controller,
                            "csrow" => num,
                        ),
                    ),
                ]);
            }
        }
    }

    Ok(metrics)
}

fn read_edac_stats(path: &Path) -> Result<(u64, u64, u64, u64), Error> {
    let ce_count = read_into(path.join("ce_count"))?;
    let ce_noinfo_count = read_into(path.join("ce_noinfo_count"))?;
    let ue_count = read_into(path.join("ue_count"))?;
    let ue_noinfo_count = read_into(path.join("ue_noinfo_count"))?;

    Ok((ce_count, ce_noinfo_count, ue_count, ue_noinfo_count))
}

fn read_edac_csrow_stats(path: &Path) -> Result<(u64, u64), Error> {
    let ce_count = read_into(path.join("ce_count"))?;
    let ue_count = read_into(path.join("ue_count"))?;

    Ok((ce_count, ue_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edac_stats() {
        let path = PathBuf::from("tests/node/sys/devices/system/edac/mc/mc0");
        let (ce_count, ce_noinfo_count, ue_count, ue_noinfo_count) =
            read_edac_stats(&path).unwrap();

        assert_eq!(ce_count, 1);
        assert_eq!(ce_noinfo_count, 2);
        assert_eq!(ue_count, 5);
        assert_eq!(ue_noinfo_count, 6);
    }

    #[test]
    fn edac_csrow_stats() {
        let path = PathBuf::from("tests/node/sys/devices/system/edac/mc/mc0/csrow0");
        let (ce_count, ue_count) = read_edac_csrow_stats(&path).unwrap();

        assert_eq!(ce_count, 3);
        assert_eq!(ue_count, 4);
    }
}
