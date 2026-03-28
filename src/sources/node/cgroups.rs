use event::{Metric, tags};

use super::{Error, Paths, read_file_no_stat};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let content = read_file_no_stat(paths.proc().join("cgroups"))?;

    let mut metrics = Vec::new();
    for summary in parse_cgroup_summaries(&content)? {
        metrics.extend([
            Metric::gauge_with_tags(
                "node_cgroups_cgroups",
                "Current cgroup number of the subsystem",
                summary.cgroups,
                tags!(
                    "subsys_name" => summary.subsystem,
                ),
            ),
            Metric::gauge_with_tags(
                "node_cgroups_enabled",
                "Current cgroup number of the subsystem",
                summary.enabled,
                tags!(
                    "subsys_name" => summary.subsystem,
                ),
            ),
        ]);
    }

    Ok(metrics)
}

/// One line of /proc/cgroups
///
/// http://man7.org/linux/man-pages/man7/cgroups.7.html
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
struct CgroupSummary<'a> {
    /// the name of the controller, also known as subsystem
    subsystem: &'a str,
    /// the unique ID of the cgroup hierarchy on which this controller is mounted
    hierarchy: u32,
    /// the number of control groups in this hierarchy using this controller
    cgroups: u32,
    /// This field contains the value 1 if this controller is enabled, or 0 if
    /// it has been disabled
    enabled: u32,
}

fn parse_cgroup_summaries(content: &str) -> Result<Vec<CgroupSummary<'_>>, Error> {
    let mut summaries = Vec::new();

    // skip header line
    for line in content.lines().skip(1) {
        let mut parts = line.split_ascii_whitespace();

        let Some(subsystem) = parts.next() else {
            return Err(Error::Malformed("cgroups stats line"));
        };

        let hierarchy = match parts.next() {
            Some(value) => value.parse()?,
            None => return Err(Error::Malformed("hierarchy field of cgroups stats")),
        };

        let cgroups = match parts.next() {
            Some(value) => value.parse()?,
            None => return Err(Error::Malformed("num_cgroups field of cgroups stats")),
        };

        let enabled = match parts.next() {
            Some(value) => value.parse()?,
            None => return Err(Error::Malformed("enabled field of cgroups stats")),
        };

        summaries.push(CgroupSummary {
            subsystem,
            hierarchy,
            cgroups,
            enabled,
        })
    }

    Ok(summaries)
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
    fn parse() {
        let content = std::fs::read_to_string("tests/node/fixtures/proc/cgroups").unwrap();
        let summaries = parse_cgroup_summaries(&content).unwrap();

        assert_eq!(summaries.len(), 12);
        assert_eq!(
            summaries[0],
            CgroupSummary {
                subsystem: "cpuset",
                hierarchy: 5,
                cgroups: 47,
                enabled: 1,
            }
        );
        assert_eq!(
            summaries[7],
            CgroupSummary {
                subsystem: "net_cls",
                hierarchy: 2,
                cgroups: 47,
                enabled: 1,
            }
        );
        assert_eq!(
            summaries[11],
            CgroupSummary {
                subsystem: "rdma",
                hierarchy: 4,
                cgroups: 1,
                enabled: 1,
            }
        )
    }
}
