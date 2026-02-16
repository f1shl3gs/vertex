//! Expose various statistics from /sys/class/powercap
//!
//! http://web.eece.maine.edu/~vweaver/projects/rapl/
//! ## RAPL(Running Average Power Limit) on Linux
//!
//! There are currently *three* ways to read RAPL results using the Linux kernel:
//! - Reading the files under /sys/class/powercap/intel-rapl/intel-rapl:0 using the powercap interface. This requires no special permissions, and was introduced in Linux 3.13
//! - Using the perf_event interface with Linux 3.14 or newer. This requires root or a paranoid less than 1 (as do all system wide measurements with -a) sudo perf stat -a -e "power/energy-cores/" /bin/ls Available events can be found via perf list or under /sys/bus/event_source/devices/power/events/
//! - Using raw-access to the underlying MSRs under /dev/msr. This requires root.
//!   Not that you cannot get readings for individual processes, the results are for the entire CPU socket.

use std::collections::BTreeMap;
use std::path::PathBuf;

use event::{Metric, tags};

use super::{Error, read_into, read_string};

/// RaplZone stores the information for one RAPL power zone
#[derive(Debug)]
struct RaplZone {
    // name of RAPL zone from file "name"
    name: String,
    // index (different value for duplicate names)
    index: i32,
    // filesystem path of RaplZone
    path: PathBuf,
    // the current microjoule value from the zone energy counter
    // // https://www.kernel.org/doc/Documentation/power/powercap/powercap.txt
    microjoules: u64,
    // max RAPL microjoule value
    max_microjoules: u64,
}

/// `get_rapl_zones` returns a slice of RaplZones
/// When RAPL files are not present, returns nil with error
/// https://www.kernel.org/doc/Documentation/power/powercap/powercap.txt
async fn get_rapl_zones(sys_path: PathBuf) -> Result<Vec<RaplZone>, Error> {
    let root = sys_path.join("class/powercap");
    let dirs = std::fs::read_dir(&root)?;

    // count name usages to avoid duplicates (label them with an index)
    let mut names: BTreeMap<String, i32> = BTreeMap::new();
    let mut zones = vec![];
    // loop through directory files searching for file "name" from subdirs
    for entry in dirs.flatten() {
        let path = entry.path();
        let Ok(name) = read_string(path.join("name")) else {
            continue;
        };

        let (name, index) = match get_name_and_index(&name) {
            Some(vs) => vs,
            None => {
                let name = &name;
                let count = names.get(name).unwrap_or(&0);
                (name.as_str(), *count)
            }
        };

        let max_microjoules = read_into(path.join("max_energy_range_uj"))?;
        let microjoules = read_into(path.join("energy_uj"))?;

        zones.push(RaplZone {
            name: name.to_string(),
            index,
            path,
            microjoules,
            max_microjoules,
        });

        // store into map how many times this name has been used. There
        // can be e.g. multiple "dram" instances without any index postfix.
        // The count is then used for indexing
        names.insert(name.to_string(), index);
    }

    Ok(zones)
}

// get_index_and_name returns a pair of (index, name) for a given name and
// it's index.
//
// the name looks like: "package-10"
fn get_name_and_index(s: &str) -> Option<(&str, i32)> {
    let p = s.find('-')?;
    let name = &s[..p];
    let next = &s[p + 1..s.len()];
    let index = next.parse().ok()?;

    Some((name, index))
}

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let zones = get_rapl_zones(sys_path).await?;
    let mut metrics = vec![];

    for zone in zones {
        metrics.push(Metric::sum_with_tags(
            format!("node_rapl_{}_joules_total", zone.name),
            format!("Current RAPL {} value in joules", zone.name),
            zone.microjoules as f64 / 1000000.0,
            tags!(
                "index" => zone.index,
                "path" => zone.path.to_string_lossy().as_ref(),
            ),
        ))
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_name_and_index() {
        assert_eq!(get_name_and_index("package-10"), Some(("package", 10)));
        assert_eq!(get_name_and_index("abc"), None);
        assert_eq!(get_name_and_index("package-"), None)
    }

    #[tokio::test]
    async fn test_get_rapl_zones() {
        let root = "tests/node/sys".into();
        let mut zones = get_rapl_zones(root).await.unwrap();

        // The readdir_r is not guaranteed to return in any specific order.
        // And the order of Github CI and Centos Stream is different, so it must be sorted
        // See: https://utcc.utoronto.ca/~cks/space/blog/unix/ReaddirOrder
        zones.sort_by(|a, b| match a.name.cmp(&b.name) {
            std::cmp::Ordering::Equal => a.index.cmp(&b.index),
            order => order,
        });

        assert_eq!(zones.len(), 3);
        assert_eq!(zones[1].microjoules, 240422366267);
        assert_eq!(zones[2].index, 10);
    }
}
