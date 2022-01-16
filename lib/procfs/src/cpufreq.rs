use crate::{read_into, Error, SysFS};
use std::path::PathBuf;

#[derive(Default, Debug, PartialEq)]
pub struct CpuFreq {
    name: String,

    pub current_frequency: Option<u64>,
    pub minimum_frequency: Option<u64>,
    pub maximum_frequency: Option<u64>,
    pub transition_latency: Option<u64>,
    pub scaling_current_frequency: Option<u64>,
    pub scaling_minimum_frequency: Option<u64>,
    pub scaling_maximum_frequency: Option<u64>,
}

impl SysFS {
    pub async fn cpufreq(&self) -> Result<Vec<CpuFreq>, Error> {
        let cpus = glob::glob(&format!(
            "{}/devices/system/cpu/cpu[0-9]*",
            self.root.to_string_lossy()
        ))?;
        let mut stats = Vec::new();

        for entry in cpus {
            let path = entry?;
            let stat = parse_cpu_freq_cpu_info(path).await?;

            stats.push(stat)
        }

        Ok(stats)
    }
}

async fn parse_cpu_freq_cpu_info(root: PathBuf) -> Result<CpuFreq, Error> {
    let mut stat = CpuFreq::default();

    // this looks terrible
    stat.name = root
        .file_name()
        .ok_or(Error::invalid_data("read cpufreq file name failed"))?
        .to_string_lossy()
        .replace("cpu", "");

    let path = root.join("cpufreq/cpuinfo_cur_freq");
    stat.current_frequency = read_into(path).await.ok();

    let path = root.join("cpufreq/cpuinfo_max_freq");
    stat.maximum_frequency = read_into(path).await.ok();

    let path = root.join("cpufreq/cpuinfo_min_freq");
    stat.minimum_frequency = read_into(path).await.ok();

    let path = root.join("cpufreq/cpuinfo_transition_latency");
    stat.transition_latency = read_into(path).await.ok();

    let path = root.join("cpufreq/scaling_cur_freq");
    stat.scaling_current_frequency = read_into(path).await.ok();

    let path = root.join("cpufreq/scaling_max_freq");
    stat.scaling_maximum_frequency = read_into(path).await.ok();

    let path = root.join("cpufreq/scaling_min_freq");
    stat.scaling_minimum_frequency = read_into(path).await.ok();

    Ok(stat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_cpu_freq_cpu_info() {
        let cpu_path = "fixtures/sys/devices/system/cpu/cpu0".into();
        let v = parse_cpu_freq_cpu_info(cpu_path).await.unwrap();
        assert_eq!(v.minimum_frequency, Some(800000));
        assert_eq!(v.maximum_frequency, Some(2400000));
    }

    #[tokio::test]
    async fn test_get_cpu_freq_stat() {
        let sysfs = SysFS::test_sysfs();
        let stats = sysfs.cpufreq().await.unwrap();

        assert_eq!(
            stats[0],
            CpuFreq {
                name: "0".to_string(),
                current_frequency: None,
                minimum_frequency: Some(800000),
                maximum_frequency: Some(2400000),
                transition_latency: Some(0),
                scaling_current_frequency: Some(1219917),
                scaling_minimum_frequency: Some(800000),
                scaling_maximum_frequency: Some(2400000),
            }
        );

        assert_eq!(
            stats[1],
            CpuFreq {
                name: "1".to_string(),
                current_frequency: Some(1200195),
                minimum_frequency: Some(1200000),
                maximum_frequency: Some(3300000),
                transition_latency: Some(4294967295),
                scaling_current_frequency: None,
                scaling_minimum_frequency: Some(1200000),
                scaling_maximum_frequency: Some(3300000),
            }
        )
    }
}
