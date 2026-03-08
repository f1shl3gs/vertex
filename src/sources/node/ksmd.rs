use event::Metric;

use super::{Error, Paths, read_into};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let root = paths.sys().join("kernel/mm/ksm");

    let mut metrics = Vec::with_capacity(9);
    for filename in [
        "full_scans",
        "merge_across_nodes",
        "pages_shared",
        "pages_sharing",
        "pages_to_scan",
        "pages_unshared",
        "pages_volatile",
        "run",
        "sleep_millisecs",
    ] {
        let mut value: f64 = read_into(root.join(filename))?;

        let name = match filename {
            "full_scans" => {
                metrics.push(Metric::sum(
                    "node_ksmd_full_scans_total",
                    format!("ksmd '{filename}' file"),
                    value,
                ));
                continue;
            }
            "sleep_millisecs" => {
                value /= 1000.0;
                "sleep_seconds"
            }
            _ => filename,
        };

        metrics.push(Metric::gauge(
            format!("node_ksmd_{name}"),
            format!("ksmd '{filename}' file"),
            value,
        ));
    }

    Ok(metrics)
}
