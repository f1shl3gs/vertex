use std::path::Path;
use tokio::io::AsyncBufReadExt;
use crate::sources::node::errors::{Error, ErrorContext};
use crate::sources::node::read_to_string;

use lazy_static::lazy_static;
use regex::Regex;

/// Exposes statistics about devices in `/proc/mdstat` (does nothing if no `/proc/mdstat` present).
pub async fn gather() {}

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
    disk_spare: i64,

    // number of blocks the device holds
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
        let state = parts[2]; // active of inactive

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
        let mut state = "";
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
                let (_sb, _pct, _finish, _speed) = eval_recovery_line(lines[sync_line_index])
                    .context("parsing sync line in md device failed")?;
                synced_blocks = _sb;
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
            disk_spare: spare,
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
//       [=>...................]  check =  5.7% (114176/1993728) finish=0.2min speed=114176K/sec
fn eval_recovery_line(line: &str) -> Result<(i64, f64, f64, f64), Error> {
    /*let caps = match recovery_line_blocks_re.captures(line) {
        Some(caps) => {
            caps.iter()
                .map(|m| m.unwrap().as_str())
                .collect::<Vec<_>>()
        }
        None => return Err(Error::new_invalid("unexpected recovery line"))
    };

    let synced_blocks = caps[1].parse()?;

    // get percentage complete
    let caps = match recovery_line_pct_re.captures(line) {
        Some(caps) => {
            caps.iter()
                .map(|m| m.unwrap().as_str())
                .collect::<Vec<_>>()
        }
        None => return Err(Error::new_invalid("unexpected recovery line"))
    };

    let pct = caps[1].parse()?;

    // get time expected left to complete

*/
    todo!()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use nom::branch::alt;
    use nom::bytes::streaming::take_while;
    use nom::character::complete::{alpha1, digit1};
    use nom::combinator::{map, map_res, opt, recognize};
    use nom::bytes::complete::tag;
    use nom::error::context;
    use nom::IResult;
    use nom::sequence::{delimited, pair, tuple};
    use super::*;

    #[test]
    fn test_captures_iter() {
        let cs = recovery_line_blocks_re.captures("xxdfadfasd");
        println!("{:?}", cs);
    }

    fn process_bar(input: &str) -> IResult<&str, &str> {
        take_while(|c| c != '[')(input)
    }

    #[test]
    fn test_parse_recovering_line() {
        let line = r#"      [=>...................]  recovery =  8.5% (16775552/195310144) finish=17.0min speed=259783K/sec"#;

        use nom::{Err, error::ErrorKind};
        use nom::sequence::tuple;
        use nom::character::complete::{alpha1, digit1};
        let mut parser = tuple((alpha1, digit1, alpha1));
        assert_eq!(parser("abc123def"), Ok(("", ("abc", "123", "def"))));
        assert_eq!(parser("123def"), Err(Err::Error(("123def", ErrorKind::Alpha))));


        let rp = tuple((process_bar));
    }
}