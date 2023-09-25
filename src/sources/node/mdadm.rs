/// Exposes statistics about devices in `/proc/mdstat` (does nothing if no `/proc/mdstat` present).
use std::path::Path;

use event::{tags, Metric};
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while};
use nom::character::complete::{digit1, multispace0};
use nom::combinator::{map_res, recognize};
use nom::number::complete::double;
use nom::IResult;
use once_cell::sync::Lazy;
use regex::Regex;

use super::{read_to_string, Error, ErrorContext};

/// MDStat holds info parsed from /proc/mdstat
#[derive(Debug, PartialEq)]
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
    let lines = content.split('\n').collect::<Vec<_>>();

    let mut stats = vec![];
    let line_count = lines.len();
    for (i, &line) in lines.iter().enumerate() {
        if line.is_empty()
            || line.starts_with("Personalities")
            || line.starts_with("unused")
            || line.starts_with(' ')
        {
            continue;
        }

        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
        if parts.len() < 3 {
            let msg = format!(
                "not enough fields in mdline(expect at least 3), line: {}",
                line
            );
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
        let (active, total, down, size) =
            eval_status_line(lines[i], lines[i + 1]).context("parse md device lines failed")?;

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
        let checking = lines[sync_line_index].contains("check");

        // Append recovery and resyncing state info
        if recovering || resyncing || checking {
            if recovering {
                state = "recovering";
            } else if checking {
                state = "checking";
            } else {
                state = "resyncing";
            }

            // Handle case when resync=PENDING or resync=DELAYED.
            if lines[sync_line_index].contains("PENDING")
                || lines[sync_line_index].contains("DELAYED")
            {
                synced_blocks = 0;
            } else {
                let (_, (_pct, _synced_blocks, _finish, _speed)) =
                    recovery_line(lines[sync_line_index]).map_err(|err| {
                        let msg = format!("parse recovery line failed, err: {}", err);
                        Error::new_invalid(msg)
                    })?;
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
            blocks_total: size,
            blocks_synced: synced_blocks,
            blocks_synced_pct: pct,
            blocks_synced_finish_time: finish,
            blocks_synced_speed: speed,
            devices: eval_component_devices(parts),
        })
    }

    Ok(stats)
}

fn eval_component_devices(fields: Vec<&str>) -> Vec<String> {
    fn parse_device_name(s: &str) -> Option<&str> {
        let bs = s.bytes();
        let mut num = false;
        for (index, b) in bs.enumerate() {
            if b == b'[' {
                return Some(&s[..index]);
            }

            if b.is_ascii_alphabetic() {
                if num {
                    return None;
                }

                continue;
            }

            if b.is_ascii_digit() {
                num = true;
                continue;
            }
        }

        None
    }

    fields
        .iter()
        .skip(4)
        .map(|s| parse_device_name(s))
        .filter(|o| o.is_some())
        .map(|o| o.unwrap().to_string())
        .collect::<Vec<_>>()
}

static STATUS_LINE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\d+) blocks .*\[(\d+)/(\d+)\] \[([U_]+)\]").unwrap());

fn eval_status_line(dev_line: &str, status_line: &str) -> Result<(i64, i64, i64, i64), Error> {
    let size_str = status_line.split_ascii_whitespace().next().unwrap();
    let size = size_str.parse().context("unexpected status line")?;

    if dev_line.contains("raid0") || dev_line.contains("linear") {
        // In the device deviceLine, only disks have a number associated with them in []
        let total = dev_line.matches('[').count() as i64;
        return Ok((total, total, 0, size));
    }

    if dev_line.contains("inactive") {
        return Ok((0, 0, 0, size));
    }

    let caps = match STATUS_LINE_RE.captures(status_line) {
        Some(caps) => caps
            .iter()
            .map(|m| m.unwrap().as_str())
            .collect::<Vec<&str>>(),
        None => vec![],
    };

    if caps.len() != 5 {
        let msg = format!("couldn't find all the substring matches {}", status_line);
        return Err(Error::new_invalid(msg));
    }

    let total = caps[2].parse()?;
    let active = caps[3].parse()?;
    let down = caps[4].matches('_').count() as i64;

    Ok((active, total, down, size))
}

// the line looks like
// [=>...................]  recovery =  8.5% (16775552/195310144) finish=17.0min speed=259783K/sec
fn recovery_line(input: &str) -> IResult<&str, (f64, i64, f64, f64)> {
    let input = input.trim();
    let (input, _) =
        take_while(|c| c == '[' || c == '=' || c == '>' || c == '.' || c == ']')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = alt((tag("recovery = "), tag("resync = "), tag("check = ")))(input)?;
    let (input, _) = multispace0(input)?;
    let (input, pct) = double(input)?;
    let (input, _) = take_while(|c: char| c == ' ' || c == '%' || c == '(')(input)?;
    let (input, synced_blocks) = map_res(recognize(digit1), str::parse)(input)?;
    let (input, _) = take_while(|c: char| c.is_ascii_digit() || c == '/')(input)?;
    let (input, _) = tag(") finish=")(input)?;
    let (input, finish) = double(input)?;
    let (input, _) = tag("min speed=")(input)?;
    let (input, speed) = double(input)?;

    Ok((input, (pct, synced_blocks, finish, speed)))
}

fn state_metric_value(key: &str, state: &str) -> f64 {
    if key == state {
        1.0
    } else {
        0.0
    }
}

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
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
            ),
        ]);
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
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
        let path = Path::new("tests/fixtures/proc/mdstat");
        let stats = parse_mdstat(path).await.unwrap();

        assert_eq!(
            stats,
            vec![
                MDStat {
                    name: "md3".to_string(),
                    activity_state: "active".to_string(),
                    disks_active: 8,
                    disks_total: 8,
                    disks_failed: 0,
                    disk_down: 0,
                    disks_spare: 2,
                    blocks_total: 5853468288,
                    blocks_synced: 5853468288,
                    blocks_synced_pct: 0.0,
                    blocks_synced_finish_time: 0.0,
                    blocks_synced_speed: 0.0,
                    devices: vec![
                        "sda1".to_string(),
                        "sdh1".to_string(),
                        "sdg1".to_string(),
                        "sdf1".to_string(),
                        "sde1".to_string(),
                        "sdd1".to_string(),
                        "sdc1".to_string(),
                        "sdb1".to_string(),
                        "sdd1".to_string(),
                        "sdd2".to_string(),
                    ],
                },
                MDStat {
                    name: "md127".to_string(),
                    activity_state: "active".to_string(),
                    disks_active: 2,
                    disks_total: 2,
                    disks_failed: 0,
                    disk_down: 0,
                    disks_spare: 0,
                    blocks_total: 312319552,
                    blocks_synced: 312319552,
                    blocks_synced_pct: 0f64,
                    blocks_synced_finish_time: 0f64,
                    blocks_synced_speed: 0f64,
                    devices: vec!["sdi2".to_string(), "sdj2".to_string()],
                },
                MDStat {
                    name: "md0".to_string(),
                    activity_state: "active".to_string(),
                    disks_active: 2,
                    disks_total: 2,
                    disks_failed: 0,
                    disk_down: 0,
                    disks_spare: 0,
                    blocks_total: 248896,
                    blocks_synced: 248896,
                    blocks_synced_pct: 0.0,
                    blocks_synced_finish_time: 0.0,
                    blocks_synced_speed: 0.0,
                    devices: vec!["sdi1".to_string(), "sdj1".to_string()],
                },
                MDStat {
                    name: "md4".to_string(),
                    activity_state: "inactive".to_string(),
                    disks_active: 0,
                    disks_total: 0,
                    disks_failed: 1,
                    disk_down: 0,
                    disks_spare: 1,
                    blocks_total: 4883648,
                    blocks_synced: 4883648,
                    blocks_synced_pct: 0.0,
                    blocks_synced_finish_time: 0.0,
                    blocks_synced_speed: 0.0,
                    devices: vec!["sda3".to_string(), "sdb3".to_string()],
                },
                MDStat {
                    name: "md6".to_string(),
                    activity_state: "recovering".to_string(),
                    disks_active: 1,
                    disks_total: 2,
                    disks_failed: 1,
                    disk_down: 1,
                    disks_spare: 1,
                    blocks_total: 195310144,
                    blocks_synced: 16775552,
                    blocks_synced_pct: 8.5,
                    blocks_synced_finish_time: 17.0,
                    blocks_synced_speed: 259783.0,
                    devices: vec!["sdb2".to_string(), "sdc".to_string(), "sda2".to_string()],
                },
                MDStat {
                    name: "md8".to_string(),
                    activity_state: "resyncing".to_string(),
                    disks_active: 2,
                    disks_total: 2,
                    disks_failed: 0,
                    disk_down: 0,
                    disks_spare: 2,
                    blocks_total: 195310144,
                    blocks_synced: 16775552,
                    blocks_synced_pct: 8.5,
                    blocks_synced_finish_time: 17.0,
                    blocks_synced_speed: 259783.0,
                    devices: vec![
                        "sdb1".to_string(),
                        "sda1".to_string(),
                        "sdc".to_string(),
                        "sde".to_string(),
                    ],
                },
                MDStat {
                    name: "md201".to_string(),
                    activity_state: "checking".to_string(),
                    disks_active: 2,
                    disks_total: 2,
                    disks_failed: 0,
                    disk_down: 0,
                    disks_spare: 0,
                    blocks_total: 1993728,
                    blocks_synced: 114176,
                    blocks_synced_pct: 5.7,
                    blocks_synced_finish_time: 0.2,
                    blocks_synced_speed: 114176.0,
                    devices: vec!["sda3".to_string(), "sdb3".to_string(),],
                },
                MDStat {
                    name: "md7".to_string(),
                    activity_state: "active".to_string(),
                    disks_active: 3,
                    disks_total: 4,
                    disks_failed: 1,
                    disk_down: 1,
                    disks_spare: 0,
                    blocks_total: 7813735424,
                    blocks_synced: 7813735424,
                    blocks_synced_pct: 0.0,
                    blocks_synced_finish_time: 0.0,
                    blocks_synced_speed: 0.0,
                    devices: vec![
                        "sdb1".to_string(),
                        "sde1".to_string(),
                        "sdd1".to_string(),
                        "sdc1".to_string(),
                    ],
                },
                MDStat {
                    name: "md9".to_string(),
                    activity_state: "resyncing".to_string(),
                    disks_active: 4,
                    disks_total: 4,
                    disks_spare: 1,
                    disk_down: 0,
                    disks_failed: 2,
                    blocks_total: 523968,
                    blocks_synced: 0,
                    blocks_synced_pct: 0.0,
                    blocks_synced_finish_time: 0.0,
                    blocks_synced_speed: 0.0,
                    devices: vec![
                        "sdc2".to_string(),
                        "sdd2".to_string(),
                        "sdb2".to_string(),
                        "sda2".to_string(),
                        "sde".to_string(),
                        "sdf".to_string(),
                        "sdg".to_string(),
                    ],
                },
                MDStat {
                    name: "md10".to_string(),
                    activity_state: "active".to_string(),
                    disks_active: 2,
                    disks_total: 2,
                    disks_failed: 0,
                    disk_down: 0,
                    disks_spare: 0,
                    blocks_total: 314159265,
                    blocks_synced: 314159265,
                    blocks_synced_pct: 0.0,
                    blocks_synced_finish_time: 0.0,
                    blocks_synced_speed: 0.0,
                    devices: vec!["sda1".to_string(), "sdb1".to_string(),],
                },
                MDStat {
                    name: "md11".to_string(),
                    activity_state: "resyncing".to_string(),
                    disks_active: 2,
                    disks_total: 2,
                    disks_failed: 1,
                    disk_down: 0,
                    disks_spare: 2,
                    blocks_total: 4190208,
                    blocks_synced: 0,
                    blocks_synced_pct: 0.0,
                    blocks_synced_finish_time: 0.0,
                    blocks_synced_speed: 0.0,
                    devices: vec![
                        "sdb2".to_string(),
                        "sdc2".to_string(),
                        "sdc3".to_string(),
                        "hda".to_string(),
                        "ssdc2".to_string(),
                    ],
                },
                MDStat {
                    name: "md12".to_string(),
                    activity_state: "active".to_string(),
                    disks_active: 2,
                    disks_total: 2,
                    disks_spare: 0,
                    disk_down: 0,
                    disks_failed: 0,
                    blocks_total: 3886394368,
                    blocks_synced: 3886394368,
                    blocks_synced_pct: 0.0,
                    blocks_synced_finish_time: 0.0,
                    blocks_synced_speed: 0.0,
                    devices: vec!["sdc2".to_string(), "sdd2".to_string(),],
                },
                MDStat {
                    name: "md126".to_string(),
                    activity_state: "active".to_string(),
                    disks_active: 2,
                    disks_total: 2,
                    disks_failed: 0,
                    disk_down: 0,
                    disks_spare: 0,
                    blocks_total: 1855870976,
                    blocks_synced: 1855870976,
                    blocks_synced_pct: 0.0,
                    blocks_synced_finish_time: 0.0,
                    blocks_synced_speed: 0.0,
                    devices: vec!["sdb".to_string(), "sdc".to_string(),],
                },
                MDStat {
                    name: "md219".to_string(),
                    activity_state: "inactive".to_string(),
                    disks_total: 0,
                    disks_failed: 0,
                    disks_active: 0,
                    disk_down: 0,
                    disks_spare: 3,
                    blocks_total: 7932,
                    blocks_synced: 7932,
                    blocks_synced_pct: 0.0,
                    blocks_synced_finish_time: 0.0,
                    blocks_synced_speed: 0.0,
                    devices: vec!["sdc".to_string(), "sda".to_string(),],
                },
                MDStat {
                    name: "md00".to_string(),
                    activity_state: "active".to_string(),
                    disks_active: 1,
                    disks_total: 1,
                    disks_failed: 0,
                    disk_down: 0,
                    disks_spare: 0,
                    blocks_total: 4186624,
                    blocks_synced: 4186624,
                    blocks_synced_pct: 0.0,
                    blocks_synced_finish_time: 0.0,
                    blocks_synced_speed: 0.0,
                    devices: vec!["xvdb".to_string(),],
                },
                MDStat {
                    name: "md120".to_string(),
                    activity_state: "active".to_string(),
                    disks_active: 2,
                    disks_total: 2,
                    disks_failed: 0,
                    disk_down: 0,
                    disks_spare: 0,
                    blocks_total: 2095104,
                    blocks_synced: 2095104,
                    blocks_synced_pct: 0.0,
                    blocks_synced_finish_time: 0.0,
                    blocks_synced_speed: 0.0,
                    devices: vec!["sda1".to_string(), "sdb1".to_string(),],
                },
                MDStat {
                    name: "md101".to_string(),
                    activity_state: "active".to_string(),
                    disks_active: 3,
                    disks_total: 3,
                    disks_failed: 0,
                    disk_down: 0,
                    disks_spare: 0,
                    blocks_total: 322560,
                    blocks_synced: 322560,
                    blocks_synced_pct: 0.0,
                    blocks_synced_finish_time: 0.0,
                    blocks_synced_speed: 0.0,
                    devices: vec!["sdb".to_string(), "sdd".to_string(), "sdc".to_string(),],
                },
            ]
        );
    }

    #[test]
    fn test_eval_component_devices() {
        let devices = eval_component_devices(vec![
            "md3",
            ":",
            "active",
            "raid6",
            "sda1[8]",
            "sdh1[7]",
            "sdg1[6]",
            "sdf1[5]",
            "sde1[11]",
            "sdd1[3]",
            "sdc1[10]",
            "sdb1[9]",
            "sdd1[10](S)",
            "sdd2[11](S)",
        ]);

        assert_eq!(
            devices,
            vec![
                "sda1".to_string(),
                "sdh1".to_string(),
                "sdg1".to_string(),
                "sdf1".to_string(),
                "sde1".to_string(),
                "sdd1".to_string(),
                "sdc1".to_string(),
                "sdb1".to_string(),
                "sdd1".to_string(),
                "sdd2".to_string(),
            ]
        )
    }

    #[test]
    fn test_eval_devices_invalid_name() {
        // md6 : active raid1 sdb2[2](F) sdc[1](S) sda2[0]
        let devices = eval_component_devices(vec![
            "md6",
            ":",
            "active",
            "raid1",
            "sdb2[2](F)",
            "sdc[1](S)",
            "sda2[0]",
        ]);

        assert_eq!(devices, vec!["sdb2", "sdc", "sda2"])
    }
}
