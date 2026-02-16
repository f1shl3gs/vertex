//! Exposes statistics from /sys/class/scsi_tape

use std::path::{Path, PathBuf};

use event::{Metric, tags};

use super::{Error, read_into};

pub async fn collect(sysfs: PathBuf) -> Result<Vec<Metric>, Error> {
    let valid_device = regex::Regex::new(r#"^st\d+$"#).unwrap();

    let root = sysfs.join("class/scsi_tape");
    let dirs = root.read_dir()?;

    let mut metrics = Vec::with_capacity(10);
    for entry in dirs.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = path.file_name().map(|name| name.to_string_lossy()).unwrap();
        if !valid_device.is_match(name.as_ref()) {
            continue;
        }

        let stats = match read_stats(&path) {
            Ok(stats) => stats,
            Err(err) => {
                warn!(message = "failed to read tape stats", ?path, ?err,);

                continue;
            }
        };

        metrics.extend([
            Metric::gauge_with_tags(
                "node_tape_io_now",
                "The number of I/Os currently outstanding to this device.",
                stats.in_flight,
                tags!("device" => name.to_string()),
            ),
            Metric::sum_with_tags(
                "node_tape_io_time_seconds_total",
                "The amount of time spent waiting for all I/O to complete (including read and write). This includes tape movement commands such as seeking between file or set marks and implicit tape movement such as when rewind on close tape devices are used.",
                stats.io_ns as f64 * 0.000000001,
                tags!("device" => name.to_string()),
            ),
            Metric::sum_with_tags(
                "node_tape_io_others_total",
                "The number of I/Os issued to the tape drive other than read or write commands. The time taken to complete these commands uses the following calculation io_time_seconds_total-read_time_seconds_total-write_time_seconds_total",
                stats.other_cnt,
                tags!("device" => name.to_string()),
            ),
            Metric::sum_with_tags(
                "node_tape_read_bytes_total",
                "The number of bytes read from the tape drive.",
                stats.read_byte_cnt,
                tags!("device" => name.to_string()),
            ),
            Metric::sum_with_tags(
                "node_tape_reads_completed_total",
                "The number of read requests issued to the tape drive.",
                stats.read_cnt,
                tags!("device" => name.to_string()),
            ),
            Metric::sum_with_tags(
                "node_tape_read_time_seconds_total",
                "The amount of time spent waiting for read requests to complete.",
                stats.read_ns as f64 * 0.000000001,
                tags!("device" => name.to_string()),
            ),
            Metric::sum_with_tags(
                "node_tape_residual_total",
                "The number of times during a read or write we found the residual amount to be non-zero. This should mean that a program is issuing a read larger thean the block size on tape. For write not all data made it to tape.",
                stats.resid_cnt,
                tags!("device" => name.to_string()),
            ),
            Metric::sum_with_tags(
                "node_tape_written_bytes_total",
                "The number of bytes written to the tape drive.",
                stats.write_byte_cnt,
                tags!("device" => name.to_string()),
            ),
            Metric::sum_with_tags(
                "node_tape_writes_completed_total",
                "The number of write requests issued to the tape drive.",
                stats.write_cnt,
                tags!("device" => name.to_string()),
            ),
            Metric::sum_with_tags(
                "node_tape_write_time_seconds_total",
                "The amount of time spent waiting for write requests to complete.",
                stats.write_ns as f64 * 0.000000001,
                tags!("device" => name.to_string()),
            )
        ]);
    }

    Ok(metrics)
}

struct Stats {
    write_ns: u64,       // /sys/class/scsi_tape/<Name>/stats/write_ns
    read_byte_cnt: u64,  // /sys/class/scsi_tape/<Name>/stats/read_byte_cnt
    io_ns: u64,          // /sys/class/scsi_tape/<Name>/stats/io_ns
    write_cnt: u64,      // /sys/class/scsi_tape/<Name>/stats/write_cnt
    resid_cnt: u64,      // /sys/class/scsi_tape/<Name>/stats/resid_cnt
    read_ns: u64,        // /sys/class/scsi_tape/<Name>/stats/read_ns
    in_flight: u64,      // /sys/class/scsi_tape/<Name>/stats/in_flight
    other_cnt: u64,      // /sys/class/scsi_tape/<Name>/stats/other_cnt
    read_cnt: u64,       // /sys/class/scsi_tape/<Name>/stats/read_cnt
    write_byte_cnt: u64, // /sys/class/scsi_tape/<Name>/stats/write_byte_cnt
}

fn read_stats(path: &Path) -> Result<Stats, Error> {
    let write_ns = read_into(path.join("stats/write_ns"))?;
    let read_byte_cnt = read_into(path.join("stats/read_byte_cnt"))?;
    let io_ns = read_into(path.join("stats/io_ns"))?;
    let write_cnt = read_into(path.join("stats/write_cnt"))?;
    let resid_cnt = read_into(path.join("stats/resid_cnt"))?;
    let read_ns = read_into(path.join("stats/read_ns"))?;
    let in_flight = read_into(path.join("stats/in_flight"))?;
    let other_cnt = read_into(path.join("stats/other_cnt"))?;
    let read_cnt = read_into(path.join("stats/read_cnt"))?;
    let write_byte_cnt = read_into(path.join("stats/write_byte_cnt"))?;

    Ok(Stats {
        write_ns,
        read_byte_cnt,
        io_ns,
        write_cnt,
        resid_cnt,
        read_ns,
        in_flight,
        other_cnt,
        read_cnt,
        write_byte_cnt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read() {
        let path = Path::new("tests/node/sys/class/scsi_tape/st0");
        let stats = read_stats(path).unwrap();

        assert_eq!(stats.write_ns, 5233597394395);
        assert_eq!(stats.read_byte_cnt, 979383912);
        assert_eq!(stats.io_ns, 9247011087720);
        assert_eq!(stats.write_cnt, 53772916);
        assert_eq!(stats.resid_cnt, 19);
        assert_eq!(stats.read_ns, 33788355744);
        assert_eq!(stats.in_flight, 1);
        assert_eq!(stats.other_cnt, 1409);
        assert_eq!(stats.read_cnt, 3741);
        assert_eq!(stats.write_byte_cnt, 1496246784000);
    }
}
