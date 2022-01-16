use crate::{Error, ProcFS};
use tokio::io::AsyncBufReadExt;

#[derive(Default)]
pub struct Diskstats {
    // Info contains identifying information for a block device such as a disk drive
    pub major: u32,
    pub minor: u32,
    pub device: String,

    // IOStats models the iostats data described in the kernel documentation
    // https://www.kernel.org/doc/Documentation/iostats.txt,
    // https://www.kernel.org/doc/Documentation/block/stat.txt,
    // and https://www.kernel.org/doc/Documentation/ABI/testing/procfs-diskstats
    // ReadIOs is the number of reads completed successfully.
    read_ios: u64,
    // ReadMerges is the number of reads merged.  Reads and writes
    // which are adjacent to each other may be merged for efficiency.
    read_merges: u64,
    // ReadSectors is the total number of sectors read successfully.
    read_sectors: u64,
    // ReadTicks is the total number of milliseconds spent by all reads.
    read_ticks: u64,
    // WriteIOs is the total number of writes completed successfully.
    write_ios: u64,
    // WriteMerges is the number of reads merged.
    write_merges: u64,
    // WriteSectors is the total number of sectors written successfully.
    write_sectors: u64,
    // WriteTicks is the total number of milliseconds spent by all writes.
    write_ticks: u64,
    // IOsInProgress is number of I/Os currently in progress.
    ios_in_progress: u64,
    // IOsTotalTicks is the number of milliseconds spent doing I/Os.
    // This field increases so long as IosInProgress is nonzero.
    ios_total_ticks: u64,
    // WeightedIOTicks is the weighted number of milliseconds spent doing I/Os.
    // This can also be used to estimate average queue wait time for requests.
    weighted_io_ticks: u64,
    // DiscardIOs is the total number of discards completed successfully.
    discard_ios: u64,
    // DiscardMerges is the number of discards merged.
    discard_merges: u64,
    // DiscardSectors is the total number of sectors discarded successfully.
    discard_sectors: u64,
    // DiscardTicks is the total number of milliseconds spent by all discards.
    discard_ticks: u64,
    // FlushRequestsCompleted is the total number of flush request completed successfully.
    flush_requests_completed: u64,
    // TimeSpentFlushing is the total number of milliseconds spent flushing.
    time_spent_flushing: u64,

    // IoStatsCount contains the number of io stats read. For kernel versions 5.5+,
    // there should be 20 fields read. For kernel versions 4.18+,
    // there should be 18 fields read. For earlier kernel versions this
    // will be 14 because the discard values are not available.
    pub io_stats_count: i32,
}

impl ProcFS {
    pub async fn diskstats(&self) -> Result<Vec<Diskstats>, Error> {
        let path = self.root.join("diskstats");
        let f = tokio::fs::File::open(path).await?;
        let reader = tokio::io::BufReader::new(f);
        let mut lines = reader.lines();

        let mut stats = vec![];
        // the content looks like this
        // 259       0 nvme1n1 1265720 211651 111856315 933217 3010548 658659 454258081 6887254 0 2712821 8188690 0 0 0 0 155133 368218
        while let Some(line) = lines.next_line().await? {
            let ds: Result<Diskstats, Error> = line.split_ascii_whitespace().enumerate().try_fold(
                Diskstats::default(),
                |mut d, (index, part)| {
                    match index {
                        0 => d.major = part.parse()?,
                        1 => d.minor = part.parse()?,
                        2 => d.device = part.to_string(),
                        3 => d.read_ios = part.parse()?,
                        4 => d.read_merges = part.parse()?,
                        5 => d.read_sectors = part.parse()?,
                        6 => d.read_ticks = part.parse()?,
                        7 => d.write_ios = part.parse()?,
                        8 => d.write_merges = part.parse()?,
                        9 => d.write_sectors = part.parse()?,
                        10 => d.write_ticks = part.parse()?,
                        11 => d.ios_in_progress = part.parse()?,
                        12 => d.ios_total_ticks = part.parse()?,
                        13 => d.weighted_io_ticks = part.parse()?,
                        14 => d.discard_ios = part.parse()?,
                        15 => d.discard_merges = part.parse()?,
                        16 => d.discard_sectors = part.parse()?,
                        17 => d.discard_ticks = part.parse()?,
                        18 => d.flush_requests_completed = part.parse()?,
                        19 => d.time_spent_flushing = part.parse()?,
                        _ => {}
                    }

                    Ok(d)
                },
            );

            match ds {
                Ok(s) => stats.push(s),
                Err(_) => {
                    // TODO: handle error
                }
            }
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_diskstats() {
        let procfs = ProcFS::test_procfs();
        let stats = procfs.diskstats().await.unwrap();

        assert_eq!(stats.len(), 52);
        // 8       4 sda4 25353629 34367650 1003337964 18492232 27448755 11134218 505696880 61593380 0 7576432 80332428
        assert_eq!(stats[28].major, 8);
        assert_eq!(stats[28].minor, 4);
        assert_eq!(stats[28].device, "sda4".to_string());
        assert_eq!(stats[28].write_ios, 27448755);
        assert_eq!(stats[28].write_ticks, 61593380);
    }
}
