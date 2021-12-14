/// Exposes error detection and correction statistics
use super::{read_into, Error, ErrorContext};
use event::{tags, Metric};

pub async fn gather(sys_path: &str) -> Result<Vec<Metric>, Error> {
    let pattern = format!("{}/devices/system/edac/mc/mc[0-9]*", sys_path);
    let paths = glob::glob(&pattern).context("find mc paths failed")?;

    let mut metrics = Vec::new();
    for entry in paths {
        match entry {
            Ok(path) => {
                let controller = path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .strip_prefix("mc")
                    .unwrap();
                let path = path.as_os_str().to_str().unwrap();

                let (ce_count, ce_noinfo_count, ue_count, ue_noinfo_count) = read_edac_stats(path)
                    .await
                    .context("read edac stats failed")?;

                metrics.push(Metric::sum_with_tags(
                    "node_edac_correctable_errors_total",
                    "Total correctable memory errors.",
                    ce_count as f64,
                    tags!(
                        "controller" => controller
                    ),
                ));
                metrics.push(Metric::sum_with_tags(
                    "node_edac_uncorrectable_errors_total",
                    "Total uncorrectable memory errors.",
                    ue_count as f64,
                    tags!(
                        "controller" => controller,
                    ),
                ));
                metrics.push(Metric::sum_with_tags(
                    "node_edac_csrow_correctable_errors_total",
                    "Total correctable memory errors for this csrow.",
                    ce_noinfo_count as f64,
                    tags!(
                        "controller" => controller,
                        "csrow" => "unknown",
                    ),
                ));
                metrics.push(Metric::sum_with_tags(
                    "node_edac_csrow_uncorrectable_errors_total",
                    "Total uncorrectable memory errors for this csrow.",
                    ue_noinfo_count as f64,
                    tags!(
                        "controller" => controller,
                        "csrow" => "unknown",
                    ),
                ));

                // for each controller, walk the csrow directories
                let csrows = glob::glob(&format!("{}/csrow[0-9]*", path))
                    .context("walk csrow directories failed")?;

                for csrow in csrows {
                    match csrow {
                        Ok(path) => {
                            // looks horrible
                            let num = path
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .strip_prefix("csrow")
                                .unwrap();
                            let path = path.to_str().unwrap();

                            match read_edac_csrow_stats(path).await {
                                Ok((ce_count, ue_count)) => {
                                    metrics.push(Metric::sum_with_tags(
                                        "node_edac_csrow_correctable_errors_total",
                                        "Total correctable memory errors for this csrow.",
                                        ce_count as f64,
                                        tags!(
                                            "controller" => controller,
                                            "csrow" => num,
                                        ),
                                    ));

                                    metrics.push(Metric::sum_with_tags(
                                        "node_edac_csrow_uncorrectable_errors_total",
                                        "Total uncorrectable memory errors for this csrow.",
                                        ue_count as f64,
                                        tags!(
                                            "controller" => controller,
                                            "csrow" => num,
                                        ),
                                    ))
                                }
                                _ => {}
                            };
                        }

                        _ => {}
                    }
                }
            }

            Err(_) => {}
        }
    }

    Ok(metrics)
}

async fn read_edac_stats(path: &str) -> Result<(u64, u64, u64, u64), Error> {
    let ce_count = read_into(format!("{}/ce_count", path)).await?;
    let ce_noinfo_count = read_into(format!("{}/ce_noinfo_count", path)).await?;
    let ue_count = read_into(format!("{}/ue_count", path)).await?;
    let ue_noinfo_count = read_into(format!("{}/ue_noinfo_count", path)).await?;

    Ok((ce_count, ce_noinfo_count, ue_count, ue_noinfo_count))
}

async fn read_edac_csrow_stats(path: &str) -> Result<(u64, u64), Error> {
    let ce_count = read_into(format!("{}/ce_count", path)).await?;
    let ue_count = read_into(format!("{}/ue_count", path)).await?;

    Ok((ce_count, ue_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_edac_stats() {
        let path = "tests/fixtures/sys/devices/system/edac/mc/mc0";
        let (ce_count, ce_noinfo_count, ue_count, ue_noinfo_count) =
            read_edac_stats(path).await.unwrap();

        assert_eq!(ce_count, 1);
        assert_eq!(ce_noinfo_count, 2);
        assert_eq!(ue_count, 5);
        assert_eq!(ue_noinfo_count, 6);
    }

    #[tokio::test]
    async fn test_read_edac_csrow_stats() {
        let path = "tests/fixtures/sys/devices/system/edac/mc/mc0/csrow0";
        let (ce_count, ue_count) = read_edac_csrow_stats(path).await.unwrap();

        assert_eq!(ce_count, 3);
        assert_eq!(ue_count, 4);
    }
}
