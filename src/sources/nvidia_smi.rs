use std::borrow::Cow;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::Command;

use event::{tags, Event, Metric};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    config::{
        default_interval, deserialize_duration, serialize_duration, ticker_from_duration, DataType,
        GenerateConfig, SourceConfig, SourceContext, SourceDescription,
    },
    sources::Source,
    Error,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NvidiaSmiConfig {
    #[serde(default = "default_interval")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    interval: chrono::Duration,

    #[serde(default = "default_smi_path")]
    path: PathBuf,
}

fn default_smi_path() -> PathBuf {
    "/usr/bin/nvidia-smi".into()
}

impl GenerateConfig for NvidiaSmiConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::to_value(Self {
            interval: default_interval(),
            path: default_smi_path(),
        })
        .unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<NvidiaSmiConfig>("nvidia_smi")
}

#[async_trait::async_trait]
#[typetag::serde(name = "nvidia_smi")]
impl SourceConfig for NvidiaSmiConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let path = self.path.clone();
        let mut ticker = ticker_from_duration(self.interval)
            .unwrap()
            .take_until(ctx.shutdown);
        let mut output = ctx.out.sink_map_err(|err| {
            error!(
                message = "Error sending nvidia smi metrics",
                %err
            )
        });

        Ok(Box::pin(async move {
            while ticker.next().await.is_some() {
                match gather(&path).await {
                    Ok(metrics) => {
                        let mut stream = futures::stream::iter(metrics).map(Event::Metric).map(Ok);

                        output.send_all(&mut stream).await?;
                    }
                    Err(err) => {
                        warn!(
                            message = "Gather metrics from nvidia smi failed",
                            %err
                        );
                    }
                }
            }

            Ok(())
        }))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "nvidia_smi"
    }
}

async fn gather(path: &PathBuf) -> Result<Vec<Metric>, Error> {
    let command = format!("{}", path.to_str().unwrap());

    let mut command = Command::new(command);
    command.args(["-q", "-x"]);

    // Pipe out stdout to the process
    command.stdout(std::process::Stdio::piped());

    let mut child = command.spawn()?;

    let stdout = child.stdout.take().ok_or_else(|| {
        std::io::Error::new(ErrorKind::Other, "Unable to take stdout of spawned process")
    })?;
    let smi: SMI = serde_xml_rs::from_reader(stdout)?;

    let mut metrics = Vec::with_capacity(smi.gpus.len() * 24);
    for (index, gpu) in smi.gpus.iter().enumerate() {
        let pstate = gpu.pstat()?;
        let tags = tags!(
            "device" => &gpu.product_name,
            "index" => index.to_string(),
            "uuid" => &gpu.uuid,
        );

        metrics.extend_from_slice(&[
            Metric::gauge_with_tags(
                "nvidia_gpu_info",
                "",
                1,
                tags!(
                    "device" => &gpu.product_name,
                    "index" => index.to_string(),
                    "uuid" => &gpu.uuid,
                    "compute_mode" => &gpu.compute_mode,
                    "driver_version" => &smi.driver_version,
                    "cuda_version" => &smi.cuda_version
                ),
            ),
            Metric::gauge_with_tags("nvidia_gpu_pstate", "", pstate, tags.clone()),
            Metric::gauge_with_tags(
                "nvidia_gpu_fan_speed_percentage",
                "",
                gpu.fan_speed.value,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_fbc_stats_session",
                "",
                gpu.fbc_stats.session_count,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_fbc_stats_average_fps",
                "",
                gpu.fbc_stats.average_fps,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_fbc_stats_average_latency",
                "",
                gpu.fbc_stats.average_latency,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_memory_free_bytes",
                "",
                gpu.fb_memory_usage.free.value * 1024.0 * 1024.0,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_memory_used_bytes",
                "",
                gpu.fb_memory_usage.used.value * 1024.0 * 1024.0,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_memory_total_bytes",
                "",
                gpu.fb_memory_usage.total.value * 1024.0 * 1024.0,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_power_draw_watts",
                "",
                gpu.power_readings.power_draw.value,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_temperature",
                "",
                gpu.temperature.gpu_temp.value,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_utilization",
                "",
                gpu.utilization.gpu_util.value,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_memory_utilization",
                "",
                gpu.utilization.memory_util.value,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_encoder_utilization",
                "",
                gpu.utilization.encoder_util.value,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_decoder_utilization",
                "",
                gpu.utilization.decoder_util.value,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_current_pcie_link_gen",
                "",
                gpu.pci.pci_gpu_link_info.pcie_gen.current_link_gen,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_pcie_current_link_width",
                "",
                gpu.pci.pci_gpu_link_info.link_widths.get_link_width(),
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_encoder_stats_session",
                "",
                gpu.encoder_stats.session_count,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_encoder_stats_average_fps",
                "",
                gpu.encoder_stats.average_fps,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_encoder_stats_average_latency",
                "",
                gpu.encoder_stats.average_latency,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_clocks_current_graphics",
                "",
                gpu.clocks.graphics_clock.value,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_clocks_current_sm",
                "",
                gpu.clocks.sm_clock.value,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_clocks_current_memory",
                "",
                gpu.clocks.mem_clock.value,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nvidia_gpu_clocks_current_video",
                "",
                gpu.clocks.video_clock.value,
                tags.clone(),
            ),
        ]);
    }

    Ok(metrics)
}

enum Unit {
    Celsius,
    MiB,
    Percentage,
    Watt,
    MHz,
}

impl ToString for Unit {
    fn to_string(&self) -> String {
        match self {
            Unit::Celsius => "C",
            Unit::MiB => "MiB",
            Unit::Percentage => "%",
            Unit::Watt => "W",
            Unit::MHz => "MHz",
        }
        .to_string()
    }
}

struct Value {
    value: f64,
    unit: Unit,
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Cow<str> = serde::__private::de::borrow_cow_str(deserializer)?;

        if let Some((value, unit)) = s.split_once(" ") {
            let unit = match unit {
                "C" => Unit::Celsius,
                "MiB" => Unit::MiB,
                "%" => Unit::Percentage,
                "W" => Unit::Watt,
                "MHz" => Unit::MHz,
                _ => return Err(serde::de::Error::custom("Unknown unit")),
            };

            let value = value.parse::<f64>().map_err(serde::de::Error::custom)?;

            Ok(Value { value, unit })
        } else {
            Err(serde::de::Error::custom("Invalid Value format"))
        }
    }
}

impl Serialize for Value {
    // helper require this implement, but it shall not call forever
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let raw = format!("{} {}", self.value, self.unit.to_string());
        serializer.serialize_str(&raw)
    }
}

// MemoryStats defines the structure of the memory portions in the smi output.
// The value looks like: "8116 MiB"
#[derive(Deserialize, Serialize)]
struct MemoryStats {
    total: Value,
    used: Value,
    free: Value,
}

// TempStats defines the structure of the temperature portion of the smi output.
#[derive(Deserialize, Serialize)]
struct TempStats {
    gpu_temp: Value,
}

// UtilizationStats defines the structure of the utilization portion of the smi output.
#[derive(Deserialize, Serialize)]
struct UtilizationStats {
    gpu_util: Value,
    memory_util: Value,
    encoder_util: Value,
    decoder_util: Value,
}

// PowerReadings defines the structure of the power_readings portion of the smi output.
#[derive(Deserialize, Serialize)]
struct PowerReadings {
    power_draw: Value,
}

// PCI defines the structure of the pci portion of the smi output
#[derive(Deserialize, Serialize)]
struct PcieGen {
    current_link_gen: i32,
}

#[derive(Deserialize, Serialize)]
struct LinkWidth {
    current_link_width: String,
}

impl LinkWidth {
    fn get_link_width(&self) -> i32 {
        let link_width = self.current_link_width.strip_suffix("x").unwrap_or("0");

        link_width.parse().unwrap_or(0)
    }
}

#[derive(Deserialize, Serialize)]
struct LinkInfo {
    pcie_gen: PcieGen,
    link_widths: LinkWidth,
}

#[derive(Deserialize, Serialize)]
struct PCI {
    pci_gpu_link_info: LinkInfo,
}

// EncoderStats defines the structure of the encoder_stats portion of the smi output
#[derive(Deserialize, Serialize)]
struct EncoderStats {
    session_count: i32,
    average_fps: i32,
    average_latency: i32,
}

// FBCStats defines the structure of the fbc_stats portion of the smi output
#[derive(Deserialize, Serialize)]
struct FBCStats {
    session_count: i32,
    average_fps: i32,
    average_latency: i32,
}

// ClockStats defines the structure of the clocks portion of the smi output
#[derive(Deserialize, Serialize)]
struct ClockStats {
    graphics_clock: Value,
    sm_clock: Value,
    mem_clock: Value,
    video_clock: Value,
}

// GPU defines the structure of the GPU portion of the smi output.
#[derive(Deserialize, Serialize)]
struct GPU {
    fan_speed: Value,
    fb_memory_usage: MemoryStats,
    performance_state: String,
    temperature: TempStats,
    product_name: String,
    uuid: String,
    compute_mode: String,
    utilization: UtilizationStats,
    power_readings: PowerReadings,
    pci: PCI,
    encoder_stats: EncoderStats,
    fbc_stats: FBCStats,
    clocks: ClockStats,
}

impl GPU {
    fn pstat(&self) -> Result<i32, Error> {
        let s = self
            .performance_state
            .strip_prefix("P")
            .ok_or("Invalid performance state")?;

        Ok(s.parse()?)
    }
}

// SMI defines the structure for the output of "nvidia-smi -q -x".
#[derive(Deserialize, Serialize)]
struct SMI {
    #[serde(rename = "gpu")]
    gpus: Vec<GPU>,
    driver_version: String,
    cuda_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read_to_string;

    #[test]
    fn test_deserialize_output() {
        let text = read_to_string("tests/fixtures/nvidia-smi.xml").unwrap();
        let smi: SMI = serde_xml_rs::from_str(&text).unwrap();
        assert_eq!(smi.driver_version, "470.82.00");
        assert_eq!(smi.gpus.len(), 1);
        assert_eq!(smi.gpus[0].compute_mode, "Default");
        assert_eq!(smi.gpus[0].product_name, "NVIDIA GeForce GTX 1070 Ti");
        assert_eq!(smi.gpus[0].pstat().unwrap(), 8);
        assert_eq!(smi.gpus[0].utilization.memory_util.value, 17.0);
    }
}
