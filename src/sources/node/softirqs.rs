use std::num::ParseIntError;
use std::path::PathBuf;

use event::{Metric, tags};

use super::{Error, Paths};

/// Softirqs represents the softirq statistics.
#[derive(Default)]
struct Softirqs {
    hi: Vec<u64>,
    timer: Vec<u64>,
    net_tx: Vec<u64>,
    net_rx: Vec<u64>,
    block: Vec<u64>,
    irq_poll: Vec<u64>,
    tasklet: Vec<u64>,
    sched: Vec<u64>,
    hr_timer: Vec<u64>,
    rcu: Vec<u64>,
}

impl Softirqs {
    fn parse(path: PathBuf) -> Result<Self, Error> {
        #[inline]
        fn parse_values(parts: &[&str]) -> Result<Vec<u64>, ParseIntError> {
            // first element is type name, e.g. 'HI', 'TIMER'
            parts[1..]
                .iter()
                .map(|s| s.parse::<u64>())
                .collect::<Result<Vec<u64>, ParseIntError>>()
        }

        let data = std::fs::read_to_string(path)?;
        let mut stat = Self::default();
        for line in data.lines() {
            let parts = line.split_ascii_whitespace().collect::<Vec<_>>();

            // require at least one cpu
            if parts.len() < 2 {
                continue;
            }

            match parts[0] {
                "HI:" => stat.hi = parse_values(&parts)?,
                "TIMER:" => stat.timer = parse_values(&parts)?,
                "NET_TX:" => stat.net_tx = parse_values(&parts)?,
                "NET_RX:" => stat.net_rx = parse_values(&parts)?,
                "BLOCK:" => stat.block = parse_values(&parts)?,
                "IRQ_POLL:" => stat.irq_poll = parse_values(&parts)?,
                "TASKLET:" => stat.tasklet = parse_values(&parts)?,
                "SCHED:" => stat.sched = parse_values(&parts)?,
                "HRTIMER:" => stat.hr_timer = parse_values(&parts)?,
                "RCU:" => stat.rcu = parse_values(&parts)?,
                _ => continue,
            }
        }

        Ok(stat)
    }
}

macro_rules! build_metrics {
    ($name: tt, $values: expr) => {
        $values.into_iter()
            .enumerate()
            .map(|(cpu, value)| {
                Metric::sum_with_tags(
                    "node_softirqs_functions_total",
                    "Softirq counts per CPU",
                    value,
                    tags!(
                        "cpu" => cpu,
                        "type" => $name
                    ),
                )
            })
    };
}

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let stat = Softirqs::parse(paths.proc().join("softirqs"))?;

    let mut metrics = Vec::with_capacity(10 * stat.hi.len());
    metrics.extend(build_metrics!("HI", stat.hi));
    metrics.extend(build_metrics!("TIMER", stat.timer));
    metrics.extend(build_metrics!("NET_TX", stat.net_tx));
    metrics.extend(build_metrics!("NET_RX", stat.net_rx));
    metrics.extend(build_metrics!("BLOCK", stat.block));
    metrics.extend(build_metrics!("IRQ_POLL", stat.irq_poll));
    metrics.extend(build_metrics!("TASKLET", stat.tasklet));
    metrics.extend(build_metrics!("SCHED", stat.sched));
    metrics.extend(build_metrics!("HRTIMER", stat.hr_timer));
    metrics.extend(build_metrics!("RCU", stat.rcu));

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let stat = Softirqs::parse("tests/node/fixtures/proc/softirqs".into()).unwrap();

        assert_eq!(stat.hi[0], 3);
        assert_eq!(stat.timer[1], 247490);
        assert_eq!(stat.net_tx[0], 2419);
        assert_eq!(stat.net_rx[1], 28694);
        assert_eq!(stat.block[1], 262755);
        assert_eq!(stat.irq_poll[0], 0);
        assert_eq!(stat.tasklet[0], 209);
        assert_eq!(stat.sched[0], 2278692);
        assert_eq!(stat.hr_timer[0], 1281);
        assert_eq!(stat.rcu[1], 532783);
    }
}
