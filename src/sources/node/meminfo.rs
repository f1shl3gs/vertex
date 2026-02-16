//! Collect metrics from /proc/meminfo

use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use event::Metric;

use super::Error;

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let infos = get_mem_info(proc_path).await?;

    let mut metrics = Vec::with_capacity(infos.len());
    for (key, value) in infos {
        let name = format!("node_memory_{key}");
        let desc = format!("Memory information field {key}");

        if key.ends_with("_total") {
            metrics.push(Metric::sum(name, desc, value));
        } else {
            metrics.push(Metric::gauge(name, desc, value));
        }
    }

    Ok(metrics)
}

async fn get_mem_info(root: PathBuf) -> std::io::Result<Vec<(&'static str, u64)>> {
    let file = std::fs::File::open(root.join("meminfo"))?;
    let mut reader = BufReader::new(file);

    let mut infos = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }

        let mut parts = line.split_ascii_whitespace();
        let Some(key) = parts.next() else {
            break;
        };
        let mut value = match parts.next() {
            Some(value) => value
                .parse::<u64>()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?,
            None => break,
        };

        if let Some(unit) = parts.next()
            && unit == "kB"
        {
            value *= 1024;
        }

        let key = match key {
            "Active:" => "Active_bytes",
            "Active(anon):" => "Active_anon_bytes",
            "Active(file):" => "Active_file_bytes",
            "AnonHugePages:" => "AnonHugePages_bytes",
            "AnonPages:" => "AnonPages_bytes",
            "Bounce:" => "Bounce_bytes",
            "Buffers:" => "Buffers_bytes",
            "Cached:" => "Cached_bytes",
            "CmaFree:" => "CmaFree_bytes",
            "CmaTotal:" => "CmaTotal_bytes",
            "CommitLimit:" => "CommitLimit_bytes",
            "Committed_AS:" => "Committed_AS_bytes",
            "DirectMap1G:" => "DirectMap1G_bytes",
            "DirectMap2M:" => "DirectMap2M_bytes",
            "DirectMap4k:" => "DirectMap4k_bytes",
            "Dirty:" => "Dirty_bytes",
            "HardwareCorrupted:" => "HardwareCorrupted_bytes",
            "Hugepagesize:" => "Hugepagesize_bytes",
            "Inactive:" => "Inactive_bytes",
            "Inactive(anon):" => "Inactive_anon_bytes",
            "Inactive(file):" => "Inactive_file_bytes",
            "KernelStack:" => "KernelStack_bytes",
            "Mapped:" => "Mapped_bytes",
            "MemAvailable:" => "MemAvailable_bytes",
            "MemFree:" => "MemFree_bytes",
            "MemTotal:" => "MemTotal_bytes",
            "Mlocked:" => "Mlocked_bytes",
            "NFS_Unstable:" => "NFS_Unstable_bytes",
            "PageTables:" => "PageTables_bytes",
            "Percpu:" => "Percpu_bytes",
            "SReclaimable:" => "SReclaimable_bytes",
            "SUnreclaim:" => "SUnreclaim_bytes",
            "Shmem:" => "Shmem_bytes",
            "ShmemHugePages:" => "ShmemHugePages_bytes",
            "ShmemPmdMapped:" => "ShmemPmdMapped_bytes",
            "Slab:" => "Slab_bytes",
            "SwapCached:" => "SwapCached_bytes",
            "SwapFree:" => "SwapFree_bytes",
            "SwapTotal:" => "SwapTotal_bytes",
            "Unevictable:" => "Unevictable_bytes",
            "VmallocChunk:" => "VmallocChunk_bytes",
            "VmallocTotal:" => "VmallocTotal_bytes",
            "VmallocUsed:" => "VmallocUsed_bytes",
            "Writeback:" => "Writeback_bytes",
            "WritebackTmp:" => "WritebackTmp_bytes",
            "Zswap:" => "Zswap_bytes",
            "Zswapped:" => "Zswapped_bytes",
            "HugePages_Total:" => "HugePages_Total",
            "HugePages_Free:" => "HugePages_Free",
            "HugePages_Rsvd:" => "HugePages_Rsvd",
            "HugePages_Surp:" => "HugePages_Surp",
            _ => continue,
        };

        infos.push((key, value));
    }

    Ok(infos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_mem() {
        let root = PathBuf::from("tests/node/proc");
        let infos = get_mem_info(root).await.unwrap();

        fn find(infos: &[(&str, u64)], key: &str) -> u64 {
            infos
                .iter()
                .find_map(|(k, v)| if *k == key { Some(*v) } else { None })
                .unwrap()
        }

        assert_eq!(find(&infos, "MemTotal_bytes"), 15666184 * 1024);
        assert_eq!(find(&infos, "DirectMap2M_bytes"), 16039936 * 1024);
        assert_eq!(find(&infos, "Active_bytes"), 6761276 * 1024);
        assert_eq!(find(&infos, "HugePages_Total"), 0);
    }
}
