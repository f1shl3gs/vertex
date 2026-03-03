use std::path::PathBuf;

use event::{Metric, tags};

use super::{Error, read_string};

pub async fn collect(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let path = proc_path.join("zoneinfo");

    let content = read_string(path)?;
    let infos = parse_zoneinfo(&content)?;

    let mut metrics = Vec::with_capacity(infos.len() * 20);
    for info in infos {
        let tags = tags!(
            "node" => info.node,
            "zone" => info.zone,
        );

        if let Some(value) = info.nr_free_pages {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_free_pages",
                "Total number of free pages in the zone",
                value,
                tags.clone(),
            ));
        }
        if let Some(value) = info.min {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_min_pages",
                "Zone watermark pages_min",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.low {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_low_pages",
                "Zone watermark pages_low",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.high {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_high_pages",
                "Zone watermark pages_high",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.scanned {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_scanned_pages",
                "Pages scanned since last reclaim",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.spanned {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_spanned_pages",
                "Total pages spanned by the zone, including holes",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.present {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_present_pages",
                "Physical pages existing within the zone",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.managed {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_managed_pages",
                "Present pages managed by the buddy system",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_active_anon {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_active_anon_pages",
                "Number of anonymous pages recently more used",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_inactive_anon {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_inactive_anon_pages",
                "Number of anonymous pages recently less used",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_isolated_anon {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_isolated_anon_pages",
                "Temporary isolated pages from anon lru",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_anon_pages {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_anon_pages",
                "Number of anonymous pages currently used by the system",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_anon_transparent_hugepages {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_anon_transparent_hugepages",
                "Number of anonymous transparent huge pages currently used by the system",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_active_file {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_active_file_pages",
                "Number of active pages with file-backing",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_inactive_file {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_inactive_file_pages",
                "Number of inactive pages with file-backing",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_isolated_file {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_isolated_file_pages",
                "Temporary isolated pages from file lru",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_file_pages {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_file_pages",
                "Number of file pages",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_slab_reclaimable {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_slab_reclaimable_pages",
                "Number of reclaimable slab pages",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_slab_unreclaimable {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_slab_unreclaimable_pages",
                "Number of unreclaimable slab pages",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_mlock_stack {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_mlock_stack_pages",
                "mlock()ed pages found and moved off LRU",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_kernel_stack {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_kernel_stacks",
                "Number of kernel stacks",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_mapped {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_mapped_pages",
                "Number of mapped pages",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_dirty {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_dirty_pages",
                "Number of dirty pages",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_writeback {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_writeback_pages",
                "Number of writeback pages",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_unevictable {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_unevictable_pages",
                "Number of unevictable pages",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_shmem {
            metrics.push(Metric::gauge_with_tags(
                "node_zoneinfo_nr_shmem_pages",
                "Number of shmem pages (included tmpfs/GEM pages)",
                value,
                tags.clone(),
            ))
        }

        if let Some(value) = info.nr_dirtied {
            metrics.push(Metric::sum_with_tags(
                "node_zoneinfo_nr_dirtied_total",
                "Page dirtyings since bootup",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.nr_written {
            metrics.push(Metric::sum_with_tags(
                "node_zoneinfo_nr_written_total",
                "Page writings since bootup",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.numa_hit {
            metrics.push(Metric::sum_with_tags(
                "node_zoneinfo_numa_hit_total",
                "Allocated in intended node",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.numa_miss {
            metrics.push(Metric::sum_with_tags(
                "node_zoneinfo_numa_miss_total",
                "Allocated in non intended node",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.numa_foreign {
            metrics.push(Metric::sum_with_tags(
                "node_zoneinfo_numa_foreign_total",
                "Was intended here, hit elsewhere",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.numa_interleave {
            metrics.push(Metric::sum_with_tags(
                "node_zoneinfo_numa_interleave_total",
                "Interleaver preferred this zone",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.numa_local {
            metrics.push(Metric::sum_with_tags(
                "node_zoneinfo_numa_local_total",
                "Allocation from local node",
                value,
                tags.clone(),
            ))
        }
        if let Some(value) = info.numa_other {
            metrics.push(Metric::sum_with_tags(
                "node_zoneinfo_numa_other_total",
                "Allocation from other node",
                value,
                tags.clone(),
            ))
        }

        for (index, value) in info.protection.iter().enumerate() {
            metrics.push(Metric::gauge_with_tags(
                format!("node_zoneinfo_protection_{index}"),
                format!("protection array {index}. field"),
                *value,
                tags.clone(),
            ));
        }
    }

    Ok(metrics)
}

// Zoneinfo holds info parsed from /proc/zoneinfo.
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Default)]
struct ZoneInfo<'a> {
    node: &'a str,
    zone: &'a str,
    nr_free_pages: Option<i64>,
    min: Option<i64>,
    low: Option<i64>,
    high: Option<i64>,
    scanned: Option<i64>,
    spanned: Option<i64>,
    present: Option<i64>,
    managed: Option<i64>,
    nr_active_anon: Option<i64>,
    nr_inactive_anon: Option<i64>,
    nr_isolated_anon: Option<i64>,
    nr_anon_pages: Option<i64>,
    nr_anon_transparent_hugepages: Option<i64>,
    nr_active_file: Option<i64>,
    nr_inactive_file: Option<i64>,
    nr_isolated_file: Option<i64>,
    nr_file_pages: Option<i64>,
    nr_slab_reclaimable: Option<i64>,
    nr_slab_unreclaimable: Option<i64>,
    nr_mlock_stack: Option<i64>,
    nr_kernel_stack: Option<i64>,
    nr_mapped: Option<i64>,
    nr_dirty: Option<i64>,
    nr_writeback: Option<i64>,
    nr_unevictable: Option<i64>,
    nr_shmem: Option<i64>,
    nr_dirtied: Option<i64>,
    nr_written: Option<i64>,
    numa_hit: Option<i64>,
    numa_miss: Option<i64>,
    numa_foreign: Option<i64>,
    numa_interleave: Option<i64>,
    numa_local: Option<i64>,
    numa_other: Option<i64>,
    protection: Vec<i64>,
}

fn parse_zoneinfo(content: &str) -> Result<Vec<ZoneInfo<'_>>, Error> {
    let mut infos = Vec::new();
    let mut info = ZoneInfo::default();

    for line in content.lines() {
        if let Some(stripped) = line.strip_prefix("Node ") {
            let parts = stripped
                .split(|c: char| c.is_ascii_whitespace() || c == ',')
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>();

            if !info.node.is_empty() || !info.zone.is_empty() {
                infos.push(info);
                info = ZoneInfo::default();
            }

            info.node = parts[0];
            info.zone = parts[2];

            continue;
        }

        let mut parts = line.split_ascii_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };

        match key {
            "nr_free_pages" => {
                info.nr_free_pages = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "min" => {
                info.min = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "low" => {
                info.low = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "high" => {
                info.high = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "scanned" => {
                info.scanned = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "spanned" => {
                info.spanned = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "present" => {
                info.present = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "managed" => {
                info.managed = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_active_anon" => {
                info.nr_active_anon = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_inactive_anon" => {
                info.nr_inactive_anon = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_isolated_anon" => {
                info.nr_isolated_anon = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_anon_pages" => {
                info.nr_anon_pages = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_anon_transparent_hugepages" => {
                info.nr_anon_transparent_hugepages = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_active_file" => {
                info.nr_active_file = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_inactive_file" => {
                info.nr_inactive_file = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_isolated_file" => {
                info.nr_isolated_file = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_file_pages" => {
                info.nr_file_pages = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_slab_reclaimable" => {
                info.nr_slab_reclaimable = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_slab_unreclaimable" => {
                info.nr_slab_unreclaimable = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_mlock_stack" => {
                info.nr_mlock_stack = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_kernel_stack" => {
                info.nr_kernel_stack = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_mapped" => {
                info.nr_mapped = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_dirty" => {
                info.nr_dirty = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_writeback" => {
                info.nr_writeback = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_unevictable" => {
                info.nr_unevictable = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_shmem" => {
                info.nr_shmem = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_dirtied" => {
                info.nr_dirtied = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "nr_written" => {
                info.nr_written = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "numa_hit" => {
                info.numa_hit = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "numa_miss" => {
                info.numa_miss = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "numa_foreign" => {
                info.numa_foreign = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "numa_interleave" => {
                info.numa_interleave = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "numa_local" => {
                info.numa_local = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "numa_other" => {
                info.numa_other = match parts.next() {
                    Some(value) => value.parse().ok(),
                    None => None,
                }
            }
            "protection:" => {
                info.protection = parts
                    .map(|x| x.trim_matches(['(', ',', ')']).parse())
                    .take(5)
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap_or_default();
            }
            _ => {}
        }
    }

    infos.push(info);

    Ok(infos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let content = std::fs::read_to_string("tests/node/fixtures/proc/zoneinfo").unwrap();
        let got = parse_zoneinfo(&content).unwrap();

        let want = [
            ZoneInfo {
                node: "0",
                zone: "DMA",
                nr_free_pages: Some(3952),
                min: Some(33),
                low: Some(41),
                high: Some(49),
                spanned: Some(4095),
                present: Some(3975),
                managed: Some(3956),
                nr_active_anon: Some(547580),
                nr_inactive_anon: Some(230981),
                nr_isolated_anon: Some(0),
                nr_anon_pages: Some(795576),
                nr_anon_transparent_hugepages: Some(0),
                nr_active_file: Some(346282),
                nr_inactive_file: Some(316904),
                nr_isolated_file: Some(0),
                nr_file_pages: Some(761874),
                nr_slab_reclaimable: Some(131220),
                nr_slab_unreclaimable: Some(47320),
                nr_kernel_stack: Some(0),
                nr_mapped: Some(215483),
                nr_dirty: Some(908),
                nr_writeback: Some(0),
                nr_unevictable: Some(115467),
                nr_shmem: Some(224925),
                nr_dirtied: Some(8007423),
                nr_written: Some(7752121),
                numa_hit: Some(1),
                numa_miss: Some(0),
                numa_foreign: Some(0),
                numa_interleave: Some(0),
                numa_local: Some(1),
                numa_other: Some(0),
                protection: vec![0, 2877, 7826, 7826, 7826],
                ..Default::default()
            },
            ZoneInfo {
                node: "0",
                zone: "DMA32",
                nr_free_pages: Some(204252),
                min: Some(19510),
                low: Some(21059),
                high: Some(22608),
                spanned: Some(1044480),
                present: Some(759231),
                managed: Some(742806),
                nr_kernel_stack: Some(2208),
                numa_hit: Some(113952967),
                numa_miss: Some(0),
                numa_foreign: Some(0),
                numa_interleave: Some(0),
                numa_local: Some(113952967),
                numa_other: Some(0),
                protection: vec![0, 0, 4949, 4949, 4949],
                ..Default::default()
            },
            ZoneInfo {
                node: "0",
                zone: "Normal",
                nr_free_pages: Some(18553),
                min: Some(11176),
                low: Some(13842),
                high: Some(16508),
                spanned: Some(1308160),
                present: Some(1308160),
                managed: Some(1268711),
                nr_kernel_stack: Some(15136),
                numa_hit: Some(162718019),
                numa_miss: Some(0),
                numa_foreign: Some(0),
                numa_interleave: Some(26812),
                numa_local: Some(162718019),
                numa_other: Some(0),
                protection: vec![0, 0, 0, 0, 0],
                ..Default::default()
            },
            ZoneInfo {
                node: "0",
                zone: "Movable",
                min: Some(0),
                low: Some(0),
                high: Some(0),
                spanned: Some(0),
                present: Some(0),
                managed: Some(0),
                protection: vec![0, 0, 0, 0, 0],
                ..Default::default()
            },
            ZoneInfo {
                node: "0",
                zone: "Device",
                min: Some(0),
                low: Some(0),
                high: Some(0),
                spanned: Some(0),
                present: Some(0),
                managed: Some(0),
                protection: vec![0, 0, 0, 0, 0],
                ..Default::default()
            },
        ];

        assert_eq!(want.as_ref(), got);
    }
}
