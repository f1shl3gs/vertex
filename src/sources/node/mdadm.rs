/// Exposes statistics about devices in `/proc/mdstat` (does nothing if no `/proc/mdstat` present).

use std::path::Path;
use tokio::io::AsyncBufReadExt;
use crate::{
    tags,
    sources::node::errors::{Error, ErrorContext},
};
use crate::sources::node::read_to_string;

use lazy_static::lazy_static;
use nom::bytes::complete::{tag, take_while};
use nom::character::complete::{digit1, multispace0};
use nom::combinator::{map_res, recognize};
use nom::IResult;
use nom::number::complete::double;
use regex::Regex;
use crate::event::Metric;

/// MDStat holds info parsed from /proc/mdstat
struct MDStat {
    // name of the device
    name: String,

    // activity-state of the device
    activity_state: String,

    // number of active disks
    disks_active: i64,

    // total number of disks the device required
    disks_total: i64,

    // number of failed disks
    disks_failed: i64,

    // number of down disks. (the _ indicator in the status line)
    disk_down: i64,

    // spare disks in the device
    disks_spare: i64,

    // number of blocks the device holds
    blocks_total: i64,

    // Number of blocks on the device that are in sync.
    blocks_synced: i64,

    // progress percentage of current sync
    blocks_synced_pct: f64,

    // estimated finishing time for current sync (in minutes)
    blocks_synced_finish_time: f64,

    // current sync speed (in Kilobytes/sec)
    blocks_synced_speed: f64,

    // name of md component device
    devices: Vec<String>,
}

async fn parse_mdstat<P: AsRef<Path>>(path: P) -> Result<Vec<MDStat>, Error> {
    let content = read_to_string(path).await?;
    let lines = content.split("\n")
        .collect::<Vec<_>>();

    let mut stats = vec![];
    let line_count = lines.len();
    for (i, &line) in lines.iter().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("Personalities") || line.starts_with("unused") {
            continue;
        }

        let parts = line.split_ascii_whitespace()
            .collect::<Vec<_>>();
        if parts.len() < 3 {
            let msg = format!("not enough fields in mdline(expect at least 3), line: {}", line);
            return Err(Error::new_invalid(msg));
        }

        let name = parts[0];
        let mut state = parts[2]; // active or inactive

        if line_count <= i + 3 {
            let msg = format!("error parsing: {}, too few lines for md device", name);
            return Err(Error::new_invalid(msg));
        }

        // failed disks have the suffix(F) & Spare disks have the suffix (S)
        let fail = line.matches("(F)").count() as i64;
        let spare = line.matches("(S)").count() as i64;
        let (active, total, down, size) = eval_status_line(lines[i], lines[i + 1])
            .context("parse md device lines failed")?;

        let mut sync_line_index = i + 2;
        if lines[i + 2].contains("bitmap") {
            // skip bitmap line
            sync_line_index += 1;
        }

        // If device is syncing at the moment, get the number of currently
        // synced bytes, otherwise that number equals the size of the device.
        let mut synced_blocks = size;
        let mut speed = 0f64;
        let mut finish = 0f64;
        let mut pct = 0f64;
        let recovering = lines[sync_line_index].contains("recovery");
        let resyncing = lines[sync_line_index].contains("resync");
        let checking = lines[sync_line_index].contains("checking");

        // Append recovery and resyncing state info
        if recovering || resyncing || checking {
            if recovering {
                state = "recovery";
            } else if checking {
                state = "checking";
            } else {
                state = "resyncing";
            }

            // Handle case when resync=PENDING or resync=DELAYED.
            if lines[sync_line_index].contains("PENDING") {
                synced_blocks = 0;
            } else {
                let (_, (_pct, _synced_blocks, _finish, _speed)) = recovery_line(line)
                    .map_err(|_| Error::new_invalid("parse recovery line failed"))?;
                synced_blocks = _synced_blocks;
                pct = _pct;
                finish = _finish;
                speed = _speed;
            }
        }

        stats.push(MDStat {
            name: name.to_string(),
            activity_state: state.to_string(),
            disks_active: active,
            disks_total: total,
            disks_failed: fail,
            disk_down: down,
            disks_spare: spare,
            // TODO: fix blocks_total
            blocks_total: 0,
            blocks_synced: synced_blocks,
            blocks_synced_pct: pct,
            blocks_synced_finish_time: finish,
            blocks_synced_speed: speed,
            devices: vec![],
        })
    }

    Ok(stats)
}

lazy_static! {
    static ref status_line_re: Regex = Regex::new(r#"(\d+) blocks .*\[(\d+)/(\d+)\] \[([U_]+)\]"#).unwrap();
    static ref recovery_line_blocks_re: Regex = Regex::new(r#"\((\d+)/\d+\)"#).unwrap();
    static ref recovery_line_pct_re: Regex = Regex::new(r#"= (.+)%"#).unwrap();
    static ref recovery_line_finish_re: Regex = Regex::new(r#"finish=(.+)min"#).unwrap();
    static ref recovery_line_speed_re: Regex = Regex::new(r#"speed=(.+)[A-Z]"#).unwrap();
    static ref component_device_re: Regex = Regex::new(r#"(.*)\[\d+\]"#).unwrap();
}

fn eval_status_line(dev_line: &str, status_line: &str) -> Result<(i64, i64, i64, i64), Error> {
    let mut active = 0;
    let mut total = 0;
    let mut down = 0;
    let mut size = 0;

    let size_str = status_line.split_ascii_whitespace().nth(0).unwrap();
    size = size_str.parse()
        .context("unexpected status line")?;

    if dev_line.contains("raid0") || dev_line.contains("linear") {
        // In the device deviceLine, only disks have a number associated with them in []
        total = dev_line.matches("[").count() as i64;
        return Ok((total, total, 0, size));
    }

    if dev_line.contains("inactive") {
        return Ok((0, 0, 0, size));
    }

    let caps = match status_line_re.captures(status_line) {
        Some(caps) => caps.iter()
            .map(|m| m.unwrap().as_str())
            .collect::<Vec<&str>>(),
        None => vec![]
    };

    if caps.len() != 5 {
        let msg = format!("couldn't find all the substring matches {}", status_line);
        return Err(Error::new_invalid(msg));
    }

    total = caps[2].parse()?;
    active = caps[3].parse()?;
    down = caps[4].matches("_").count() as i64;

    Ok((active, total, down, size))
}

// the line looks like
// [=>...................]  recovery =  8.5% (16775552/195310144) finish=17.0min speed=259783K/sec
fn recovery_line(input: &str) -> IResult<&str, (f64, i64, f64, f64)> {
    let (input, _) = take_while(|c| c == '[' || c == '=' || c == '>' || c == '.' || c == ']')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("recovery = ")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, pct) = double(input)?;
    let (input, _) = take_while(|c: char| c == ' ' || c == '%' || c == '(')(input)?;
    let (input, synced_blocks) = map_res(recognize(digit1), str::parse)(input)?;
    let (input, _) = take_while(|c: char| c.is_digit(10) || c == '/')(input)?;
    let (input, _) = tag(") finish=")(input)?;
    let (input, finish) = double(input)?;
    let (input, _) = tag("min speed=")(input)?;
    let (input, speed) = double(input)?;

    Ok((input, (pct, synced_blocks, finish, speed)))
}

fn state_metric_value(key: &str, state: &str) -> f64 {
    if key == state {
        return 1.0;
    }

    return 0.0;
}

async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
    let path = Path::new(proc_path).join("mdstat");
    let stats = parse_mdstat(path).await?;

    let mut metrics = vec![];
    for stat in stats {
        let device = &stat.name;
        let state = &stat.activity_state;

        metrics.extend_from_slice(&[
            Metric::gauge_with_tags(
                "node_md_disks_required",
                "Total number of disks of device.",
                stat.disks_total as f64,
                tags!(
                    "device" => device,
                ),
            ),
            Metric::gauge_with_tags(
                "node_md_disks",
                "Number of active/failed/spare disks of device.",
                stat.disks_active as f64,
                tags!(
                    "device" => device,
                    "state" => "active"
                ),
            ),
            Metric::gauge_with_tags(
                "node_md_disks",
                "Number of active/failed/spare disks of device.",
                stat.disks_failed as f64,
                tags!(
                    "device" => device,
                    "state" => "failed"
                ),
            ),
            Metric::gauge_with_tags(
                "node_md_disks",
                "Number of active/failed/spare disks of device.",
                stat.disks_spare as f64,
                tags!(
                    "device" => device,
                    "state" => "spare"
                ),
            ),
            Metric::gauge_with_tags(
                "node_md_state",
                "Indicates the state of md-device.",
                state_metric_value("active", state),
                tags!(
                    "device" => device,
                    "state" => "active"
                ),
            ),
            Metric::gauge_with_tags(
                "node_md_state",
                "Indicates the state of md-device.",
                state_metric_value("inactive", state),
                tags!(
                    "device" => device,
                    "state" => "inactive"
                ),
            ),
            Metric::gauge_with_tags(
                "node_md_state",
                "Indicates the state of md-device.",
                state_metric_value("recovering", state),
                tags!(
                    "device" => device,
                    "state" => "recovering"
                ),
            ),
            Metric::gauge_with_tags(
                "node_md_state",
                "Indicates the state of md-device.",
                state_metric_value("resyncing", state),
                tags!(
                    "device" => device,
                    "state" => "resyncing"
                ),
            ),
            Metric::gauge_with_tags(
                "node_md_state",
                "Indicates the state of md-device.",
                state_metric_value("checking", state),
                tags!(
                    "device" => device,
                    "state" => "checking"
                ),
            ),
            Metric::gauge_with_tags(
                "node_md_blocks",
                "Total number of blocks on device.",
                stat.blocks_total as f64,
                tags!(
                    "device" => device
                ),
            ),
            Metric::gauge_with_tags(
                "node_md_blocks_synced",
                "Number of blocks synced on device.",
                stat.blocks_synced as f64,
                tags!(
                    "device" => device
                ),
            )
        ]);
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use nom::branch::alt;
    use nom::bytes::streaming::take_while;
    use nom::character::complete::{alpha1, digit1, multispace0};
    use nom::combinator::{complete, map, map_res, opt, recognize};
    use nom::bytes::complete::tag;
    use nom::error::context;
    use nom::{AsChar, IResult};
    use nom::number::complete::{float, self, double};
    use nom::sequence::{delimited, pair, tuple};
    use super::*;

    #[test]
    fn test_parse_recovering_line() {
        let line = r#"[=>...................]  recovery =  8.5% (16775552/195310144) finish=17.0min speed=259783K/sec"#;

        let (_, (pct, synced_blocks, finish, speed)) = recovery_line(line).unwrap();
        assert_eq!(pct, 8.5);
        assert_eq!(synced_blocks, 16775552);
        assert_eq!(finish, 17.0);
        assert_eq!(speed, 259783.0);
    }

    #[tokio::test]
    async fn test_parse_mdstat() {
        let path = Path::new("testdata/proc/mdstat");
        let stats = parse_mdstat(path).await.unwrap();
        assert_ne!(stats.len(), 0);
    }
}