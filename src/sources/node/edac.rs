//! Exposes error detection and correction statistics

use std::path::PathBuf;

use event::{Metric, tags};

use super::{Error, Paths, read_into};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let root = paths.sys().join("devices/system/edac/mc");

    let mut metrics = Vec::new();
    for entry in root.read_dir()?.flatten() {
        // 1. it must be a directory
        let Ok(typ) = entry.file_type() else { continue };
        if !typ.is_dir() {
            continue;
        }

        // 2. the directory's filename must be `mc[0-9]+`
        let filename = entry.file_name();
        let filename = filename.to_string_lossy();
        let Some(stripped) = filename.strip_prefix("mc") else {
            continue;
        };
        let Ok(controller) = stripped.parse::<u32>() else {
            continue;
        };

        let (ce, ce_noinfo, ue, ue_noinfo) = read_edac_stats(entry.path())?;
        metrics.extend([
            Metric::sum_with_tags(
                "node_edac_correctable_errors_total",
                "Total correctable memory errors.",
                ce,
                tags!(
                    "controller" => controller
                ),
            ),
            Metric::sum_with_tags(
                "node_edac_uncorrectable_errors_total",
                "Total uncorrectable memory errors.",
                ue,
                tags!(
                    "controller" => controller
                ),
            ),
            Metric::sum_with_tags(
                "node_edac_csrow_correctable_errors_total",
                "Total correctable memory errors for this csrow.",
                ce_noinfo,
                tags!(
                    "controller" => controller,
                    "csrow" => "unknown",
                ),
            ),
            Metric::sum_with_tags(
                "node_edac_csrow_uncorrectable_errors_total",
                "Total uncorrectable memory errors for this csrow.",
                ue_noinfo,
                tags!(
                    "controller" => controller,
                    "csrow" => "unknown",
                ),
            ),
        ]);

        // for each controller, walk the csrow directories
        for entry in entry.path().read_dir()?.flatten() {
            // 1. it must be a directory
            let Ok(typ) = entry.file_type() else { continue };
            if !typ.is_dir() {
                continue;
            }

            // 2. the directory's filename must be `csrow[0-9]+`
            let filename = entry.file_name();
            let filename = filename.to_string_lossy();
            let Some(stripped) = filename.strip_prefix("csrow") else {
                continue;
            };
            let Ok(num) = stripped.parse::<u32>() else {
                continue;
            };

            match read_edac_csrow_stats(entry.path()) {
                Ok((ce, ue)) => {
                    metrics.extend([
                        Metric::sum_with_tags(
                            "node_edac_csrow_correctable_errors_total",
                            "Total correctable memory errors for this csrow.",
                            ce,
                            tags!(
                                "controller" => controller,
                                "csrow" => num,
                            ),
                        ),
                        Metric::sum_with_tags(
                            "node_edac_csrow_uncorrectable_errors_total",
                            "Total uncorrectable memory errors for this csrow.",
                            ue,
                            tags!(
                                "controller" => controller,
                                "csrow" => num,
                            ),
                        ),
                    ]);
                }
                Err(err) => {
                    debug!(message = "failed to read edac csrow stats", path = ?entry.path(), ?err);
                    return Err(err);
                }
            }
        }
    }

    Ok(metrics)
}

fn read_edac_stats(root: PathBuf) -> Result<(u64, u64, u64, u64), Error> {
    let ce = read_into(root.join("ce_count"))?;
    let ce_noinfo = read_into(root.join("ce_noinfo_count"))?;
    let ue = read_into(root.join("ue_count"))?;
    let ue_noinfo = read_into(root.join("ue_noinfo_count"))?;

    Ok((ce, ce_noinfo, ue, ue_noinfo))
}

fn read_edac_csrow_stats(path: PathBuf) -> Result<(u64, u64), Error> {
    let ce = read_into(path.join("ce_count"))?;
    let ue = read_into(path.join("ue_count"))?;

    Ok((ce, ue))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();
        assert_ne!(metrics.len(), 0);
    }

    #[test]
    fn edac_stats() {
        let path = PathBuf::from("tests/node/fixtures/sys/devices/system/edac/mc/mc0");
        let (ce, ce_noinfo, ue, ue_noinfo) = read_edac_stats(path).unwrap();

        assert_eq!(ce, 1);
        assert_eq!(ce_noinfo, 2);
        assert_eq!(ue, 5);
        assert_eq!(ue_noinfo, 6);
    }

    #[test]
    fn edac_csrow_stats() {
        let path = PathBuf::from("tests/node/fixtures/sys/devices/system/edac/mc/mc0/csrow0");
        let (ce, ue) = read_edac_csrow_stats(path).unwrap();

        assert_eq!(ce, 3);
        assert_eq!(ue, 4);
    }
}
