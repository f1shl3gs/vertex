use event::{Metric, tags};

use super::{Error, Paths, read_into};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let root = paths.sys().join("fs/resctrl");

    let mut metrics = Vec::new();

    let dirs = std::fs::read_dir(root.join("mon_data"))?;
    for entry in dirs.flatten() {
        let file_name = entry.file_name();
        let filename = file_name.to_string_lossy();
        let Some((resource, id)) = parse_resource_and_id(filename.as_ref()) else {
            continue;
        };

        // The metrics describe the last level cache, so only L3 fits them.
        if resource != "L3" {
            continue;
        }

        if let Ok(value) = read_into::<_, u64, _>(entry.path().join("llc_occupancy")) {
            metrics.push(Metric::gauge_with_tags(
                "node_resctrl_llc_occupancy_bytes",
                "Last level cache bytes occupied in this domain.",
                value,
                tags!("domain" => id),
            ))
        }

        if let Ok(value) = read_into::<_, u64, _>(entry.path().join("mbm_local_bytes")) {
            metrics.push(Metric::sum_with_tags(
                "node_resctrl_memory_bandwidth_bytes_total",
                "Bytes moved between the last level cache and memory. scope=total includes remote sockets, scope=local is memory attached to this domain.",
                value,
                tags!("domain" => id, "scope" => "local")
            ))
        }

        if let Ok(value) = read_into::<_, u64, _>(entry.path().join("mbm_total_bytes")) {
            metrics.push(Metric::sum_with_tags(
                "node_resctrl_memory_bandwidth_bytes_total",
                "Bytes moved between the last level cache and memory. scope=total includes remote sockets, scope=local is memory attached to this domain.",
                value,
                tags!("domain" => id, "scope" => "total")
            ))
        }
    }

    Ok(metrics)
}

#[inline]
fn parse_resource_and_id(name: &str) -> Option<(&str, &str)> {
    name.strip_prefix("mon_")?.split_once("_")
}
