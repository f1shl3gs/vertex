use std::collections::HashMap;
use std::path::{Path, PathBuf};

use event::{Metric, tags};

use super::{Error, read_into, read_string};

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let (procs, threads) = get_procs_and_threads(&proc_path).await?;
    let mut metrics = vec![];

    let max_threads: usize = read_into(proc_path.join("sys/kernel/threads-max"))?;
    metrics.push(Metric::gauge(
        "node_processes_max_threads",
        "Limit of threads in the system",
        max_threads,
    ));

    let max_processes: usize = read_into(proc_path.join("sys/kernel/pid_max"))?;
    metrics.push(Metric::gauge(
        "node_processes_max_processes",
        "Number of max PIDs limit",
        max_processes,
    ));

    metrics.push(Metric::gauge(
        "node_processes_pids",
        "Number of PIDs",
        procs.total(),
    ));
    for (k, v) in procs.0 {
        metrics.push(Metric::gauge_with_tags(
            "node_processes_state",
            "Number of processes in each state",
            v,
            tags!(
                "state" => k,
            ),
        ));
    }

    metrics.push(Metric::gauge(
        "node_processes_threads",
        "Allocated threads in system",
        threads.total(),
    ));
    for (state, value) in threads.0 {
        metrics.push(Metric::gauge_with_tags(
            "node_processes_threads_state",
            "Number of threads in each state",
            value,
            tags!(
                "state" => state
            ),
        ));
    }

    Ok(metrics)
}

#[derive(Debug, Default)]
struct Stats(HashMap<String, usize>);

impl Stats {
    fn new() -> Self {
        Stats(Default::default())
    }

    #[cfg(test)]
    fn clear(&mut self) {
        self.0.iter_mut().for_each(|(_, v)| {
            *v = 0;
        })
    }

    fn total(&self) -> usize {
        self.0.iter().fold(0usize, |acc, (_, v)| acc + *v)
    }

    fn append(&mut self, s: &str) {
        match self.0.get_mut(s) {
            Some(v) => *v += 1,
            None => {
                self.0.insert(s.to_string(), 1);
            }
        }
    }
}

async fn get_procs_and_threads(root: &Path) -> Result<(Stats, Stats), Error> {
    let dirs = std::fs::read_dir(root)?;

    let mut procs = Stats::new();
    let mut threads = Stats::new();

    for entry in dirs.flatten() {
        let path = entry.path().join("stat");

        match read_string(path) {
            Ok(content) => match parse_state(&content) {
                Some(state) => procs.append(state),
                None => continue,
            },

            Err(_) => continue,
        }

        match std::fs::read_dir(entry.path().join("task")) {
            Ok(dirs) => {
                for entry in dirs.flatten() {
                    match read_string(entry.path().join("stat")) {
                        Ok(content) => match parse_state(&content) {
                            Some(state) => threads.append(state),
                            None => continue,
                        },
                        Err(_) => continue,
                    }
                }
            }
            Err(_) => continue,
        }
    }

    Ok((procs, threads))
}

fn parse_state(content: &str) -> Option<&str> {
    // Check the following resources for the details about the particular stat
    // fields and their data types:
    // * https://man7.org/linux/man-pages/man5/proc.5.html
    // * https://man7.org/linux/man-pages/man3/scanf.3.html
    let index = content.rfind(')')?;

    content.get(index + 2..index + 3)
}

// Check the following resources for the details about the particular stat
// fields and their data types:
// * https://man7.org/linux/man-pages/man5/proc.5.html
// * https://man7.org/linux/man-pages/man3/scanf.3.html
fn parse_stat_and_threads(content: &str) -> Option<(&str, usize)> {
    let index = content.rfind(')')?;

    let (_, s) = content.split_at(index + 1);
    let list = s.trim().split_ascii_whitespace().collect::<Vec<_>>();

    let state = match list.first() {
        Some(s) => *s,
        None => return None,
    };

    let threads = match list.get(17) {
        Some(s) => match s.parse::<usize>() {
            Ok(n) => n,
            Err(_) => return None,
        },
        None => return None,
    };

    Some((state, threads))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_stats() {
        let mut stats = Stats::new();
        assert_eq!(stats.total(), 0usize);

        stats.append("R");
        assert_eq!(stats.total(), 1);

        stats.append("R");
        assert_eq!(stats.total(), 2);

        stats.append("S");
        assert_eq!(stats.total(), 3);
        assert_eq!(stats.0.len(), 2);

        stats.clear();
        assert_eq!(stats.total(), 0);
        assert_eq!(stats.0.len(), 2);
    }

    #[test]
    fn test_parse_state() {
        let input = r#"26231 (vim) R 5392 7446 5392 34835 7446 4218880 32533 309516 26 82 1677 44 158 99 20 0 1 0 82375 56274944 1981 18446744073709551615 4194304 6294284 140736914091744 140736914087944 139965136429984 0 0 12288 1870679807 0 0 0 17 0 0 0 31 0 0 8391624 8481048 16420864 140736914093252 140736914093279 140736914093279 140736914096107 0"#;

        let index = input.rfind(')').unwrap();
        let c = input.get(index + 2..index + 3);

        assert_eq!(c, Some("R"));
    }
}
