//! exposing /sys/class/drm/card?/device stats.
//!
//! Expose GPU metrics using sysfs/drm.
//! amdgpu is the only driver which exposes this information through DRM.
//!
//! https://github.com/prometheus/node_exporter/pull/1998
use event::{tags, Metric};

use super::{read_into, read_to_string, Error, ErrorContext};

pub async fn gather(sys_path: &str) -> Result<Vec<Metric>, Error> {
    let stats = class_drm_card_amdgpu_stats(sys_path)
        .await
        .context("read drm amdgpu stats failed")?;

    let mut metrics = Vec::with_capacity(8 * stats.len());
    for stat in stats {
        let card = &stat.name.clone();
        let memory_vendor = &stat.memory_vram_vendor.clone();
        let power_performance_level = &stat.power_dpm_force_performance_level.clone();
        let unique_id = &stat.unique_id.clone();
        let vendor = "amd";

        metrics.extend_from_slice(&[
            Metric::gauge_with_tags(
                "node_drm_card_info",
                "Card information",
                1f64,
                tags!(
                    "card" => card,
                    "memory_vendor" => memory_vendor,
                    "power_performance_level" => power_performance_level,
                    "unique_id" => unique_id,
                    "vendor" => vendor,
                ),
            ),
            Metric::gauge_with_tags(
                "node_drm_gpu_busy_percent",
                "How busy the GPU is as a percentage.",
                stat.gpu_busy_percent,
                tags!(
                    "card" => card,
                ),
            ),
            Metric::gauge_with_tags(
                "node_drm_memory_gtt_size_bytes",
                "The size of the graphics translation table (GTT) block in bytes",
                stat.memory_gtt_size,
                tags!(
                    "card" => card,
                ),
            ),
            Metric::gauge_with_tags(
                "node_drm_memory_gtt_used_bytes",
                "The used amount of the graphics translation table (GTT) block in bytes",
                stat.memory_gtt_used,
                tags!(
                    "card" => card,
                ),
            ),
            Metric::gauge_with_tags(
                "node_drm_vis_vram_size_bytes",
                "The size of visible VRAM in bytes",
                stat.memory_visible_vram_size,
                tags!(
                    "card" => card,
                ),
            ),
            Metric::gauge_with_tags(
                "node_drm_vis_vram_used_bytes",
                "The used amount of visible VRAM in bytes",
                stat.memory_visible_vram_used,
                tags!(
                    "card" => card,
                ),
            ),
            Metric::gauge_with_tags(
                "node_drm_memory_vram_size_bytes",
                "The size of VRAM in bytes",
                stat.memory_vram_size,
                tags!(
                    "card" => card,
                ),
            ),
            Metric::gauge_with_tags(
                "node_drm_memory_vram_used_bytes",
                "The used amount of VRAM in bytes",
                stat.memory_vram_used,
                tags!(
                    "card" => card,
                ),
            ),
        ])
    }

    Ok(metrics)
}

async fn class_drm_card_amdgpu_stats(
    sys_path: &str,
) -> Result<Vec<ClassDRMCardAMDGPUStats>, Error> {
    let pattern = format!("{}/class/drm/card[0-9]", sys_path);
    let paths = glob::glob(&pattern).context("glob drm failed")?;

    let mut stats = Vec::new();
    for path in paths.flatten() {
        let card = path.to_str().unwrap();
        if let Ok(stat) = parse_class_drm_amdgpu_card(card).await {
            stats.push(stat);
        };
    }

    Ok(stats)
}

async fn read_drm_card_field(card: &str, field: &str) -> Result<u64, Error> {
    let path = format!("{}/device/{}", card, field);
    read_into(path).await
}

/// ClassDRMCardAMDGPUStats contains info from files in
/// /sys/class/drm/card<card>/device for a single amdgpu card.
/// Not all cards expose all metrics.
/// https://www.kernel.org/doc/html/latest/gpu/amdgpu.html
#[derive(Debug, PartialEq)]
struct ClassDRMCardAMDGPUStats {
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

async fn parse_class_drm_amdgpu_card(card: &str) -> Result<ClassDRMCardAMDGPUStats, Error> {
    let path = format!("{}/device/uevent", card);
    let uevent = read_to_string(path).await?;

    if !uevent.contains("DRIVER=amdgpu") {
        return Err(Error::new_invalid("the device is not an amdgpu"));
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
        .unwrap_or_default()
        .trim()
        .to_string();
    let path = format!("{}/device/power_dpm_force_performance_level", card);
    let power_dpm_force_performance_level = read_to_string(path)
        .await
        .unwrap_or_default()
        .trim()
        .to_string();
    let path = format!("{}/device/unique_id", card);
    let unique_id = read_to_string(path)
        .await
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_class_dram_card_amdgpu_stats() {
        let path = "tests/fixtures/sys";

        let stats = class_drm_card_amdgpu_stats(path).await.unwrap();

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
