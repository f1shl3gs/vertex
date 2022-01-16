use crate::{read_into, read_to_string, Error, SysFS};

/// ClassDRMCardAMDGPUStats contains info from files in
/// /sys/class/drm/card<card>/device for a single amdgpu card.
/// Not all cards expose all metrics.
/// https://www.kernel.org/doc/html/latest/gpu/amdgpu.html
#[derive(Debug, PartialEq)]
pub struct ClassDRMCardAMDGPUStats {
    // The card name
    name: String,

    // How busy the GPU is as a percentag.
    gpu_busy_percent: u64,

    // The size of the graphics translation table (GTT) block in bytes
    memory_gtt_size: u64,

    // The used amount of the graphics translation table (GTT) block in bytes.
    memory_gtt_used: u64,

    // The size of visible VRAM in bytes
    memory_visible_vram_size: u64,

    // The use amount of visible VRAM in bytes
    memory_visible_vram_used: u64,

    // The size of VRAM in bytes.
    memory_vram_size: u64,

    // The used amount of VRAM in bytes.
    memory_vram_used: u64,

    // The VRAM vendor name.
    memory_vram_vendor: String,

    // The current power performance level
    power_dpm_force_performance_level: String,

    // The unique ID of the GPU that will persist from machine to machine
    unique_id: String,
}

impl SysFS {
    pub async fn drm(&self) -> Result<Vec<ClassDRMCardAMDGPUStats>, Error> {
        let pattern = format!("{}/class/drm/card[0-9]", self.root.to_string_lossy());
        let paths = glob::glob(&pattern)?;
        let mut stats = Vec::new();

        for path in paths {
            match path {
                Ok(path) => {
                    let card = path.to_str().unwrap();
                    if let Ok(stat) = parse_class_drm_amdgpu_card(card).await {
                        stats.push(stat);
                    };
                }
                _ => {}
            }
        }

        Ok(stats)
    }
}

async fn parse_class_drm_amdgpu_card(card: &str) -> Result<ClassDRMCardAMDGPUStats, Error> {
    let path = format!("{}/device/uevent", card);
    let uevent = read_to_string(path).await?;

    if !uevent.contains("DRIVER=amdgpu") {
        return Err(Error::invalid_data("the device is not an amdgpu"));
    }

    let name = &card[card.len() - 5..];
    let gpu_busy_percent = read_drm_card_field(card, "gpu_busy_percent")
        .await
        .unwrap_or(0);
    let memory_gtt_size = read_drm_card_field(card, "mem_info_gtt_total")
        .await
        .unwrap_or(0);
    let memory_gtt_used = read_drm_card_field(card, "mem_info_gtt_used")
        .await
        .unwrap_or(0);
    let memory_visible_vram_size = read_drm_card_field(card, "mem_info_vis_vram_total")
        .await
        .unwrap_or(0);
    let memory_visible_vram_used = read_drm_card_field(card, "mem_info_vis_vram_used")
        .await
        .unwrap_or(0);
    let memory_vram_size = read_drm_card_field(card, "mem_info_vram_total")
        .await
        .unwrap_or(0);
    let memory_vram_used = read_drm_card_field(card, "mem_info_vram_used")
        .await
        .unwrap_or(0);

    let path = format!("{}/device/mem_info_vram_vendor", card);
    let memory_vram_vendor = read_to_string(path)
        .await
        .unwrap_or("".to_string())
        .trim()
        .to_string();
    let path = format!("{}/device/power_dpm_force_performance_level", card);
    let power_dpm_force_performance_level = read_to_string(path)
        .await
        .unwrap_or("".to_string())
        .trim()
        .to_string();
    let path = format!("{}/device/unique_id", card);
    let unique_id = read_to_string(path)
        .await
        .unwrap_or("".to_string())
        .trim()
        .to_string();

    Ok(ClassDRMCardAMDGPUStats {
        name: name.to_string(),
        gpu_busy_percent,
        memory_gtt_size,
        memory_gtt_used,
        memory_visible_vram_size,
        memory_visible_vram_used,
        memory_vram_size,
        memory_vram_used,
        memory_vram_vendor,
        power_dpm_force_performance_level,
        unique_id,
    })
}

async fn read_drm_card_field(card: &str, field: &str) -> Result<u64, Error> {
    let path = format!("{}/device/{}", card, field);
    read_into(path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_class_dram_card_amdgpu_stats() {
        let sysfs = SysFS::test_sysfs();
        let stats = sysfs.drm().await.unwrap();

        assert_eq!(stats.len(), 1);
        assert_eq!(
            stats[0],
            ClassDRMCardAMDGPUStats {
                name: "card0".to_string(),
                gpu_busy_percent: 4,
                memory_gtt_size: 8573157376,
                memory_gtt_used: 144560128,
                memory_visible_vram_size: 8573157376,
                memory_visible_vram_used: 1490378752,
                memory_vram_size: 8573157376,
                memory_vram_used: 1490378752,
                memory_vram_vendor: "samsung".to_string(),
                power_dpm_force_performance_level: "manual".to_string(),
                unique_id: "0123456789abcdef".to_string(),
            }
        )
    }
}
