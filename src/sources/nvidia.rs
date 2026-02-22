use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use configurable::configurable_component;
use event::{Metric, tags};
use framework::Source;
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

fn default_smi_path() -> PathBuf {
    "/usr/bin/nvidia-smi".into()
}

/// Collect metrics of NVIDIA GPU, `nvidia_smi` is installed automatically
/// if NVIDIA GPU driver installed already.
#[configurable_component(source, name = "nvidia")]
struct Config {
    /// The nvidia_smi's absolutely path.
    #[serde(default = "default_smi_path")]
    path: PathBuf,

    /// You can find out possible fields by running `nvidia-smi --help-query-gpu`
    ///
    /// The value `%s` will automatically detect the fields to query
    #[serde(default)]
    query_fields: Vec<String>,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "nvidia")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let path = self.path.clone();
        let interval = self.interval;
        let SourceContext {
            mut output,
            mut shutdown,
            ..
        } = cx;

        let infos = load_query_field(&path).await?;
        let query_fields = if self.query_fields.is_empty() {
            infos
        } else {
            self.query_fields
                .iter()
                .filter_map(|qn| match infos.iter().find(|(name, _)| name == qn) {
                    Some((_, desc)) => Some((qn.to_string(), desc.clone())),
                    None => {
                        warn!(message = "unknown query field", query_field = qn);

                        None
                    }
                })
                .collect::<Vec<_>>()
        };

        Ok(Box::pin(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    biased;

                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                let (mut metrics, code) = match scrape(&path, &query_fields).await {
                    Ok(metrics) => (metrics, 0),
                    Err(err) => {
                        warn!(message = "Gather metrics from nvidia smi failed", ?err);

                        let code = match err {
                            Error::Io(_) => -1,
                            Error::Exit(exit_code, _) => exit_code.code().unwrap_or(-1),
                        };

                        (vec![], code)
                    }
                };

                metrics.push(Metric::gauge(
                    "nvidia_command_exit_code",
                    "Exit code of the last scrape command",
                    code,
                ));

                if let Err(_err) = output.send(metrics).await {
                    error!(message = "Error sending nvidia smi metrics");

                    break;
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Io(std::io::Error),

    #[error("exec command failed(0), (1)")]
    Exit(ExitStatus, String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

async fn load_query_field(path: &Path) -> Result<Vec<(String, String)>, Error> {
    let mut cmd = tokio::process::Command::new(path);
    let mut child = cmd
        .arg("--help-query-gpu")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let status = child.wait().await?;

    if !status.success() {
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| std::io::Error::other("Unable to take stderr of spawned process"))?;

        let mut output = String::new();
        stderr.read_to_string(&mut output).await?;

        return Err(Error::Exit(status, output));
    }

    let mut columns = Vec::new();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("Unable to take stdout of spawned process"))?;
    let mut lines = BufReader::new(stdout).lines();
    loop {
        // new metric line
        let Some(line) = lines.next_line().await? else {
            break;
        };
        let line = line.trim();
        let Some(line) = line.strip_prefix('"') else {
            continue;
        };

        let Some((name, _)) = line.split_once('"') else {
            continue;
        };

        // description, one line only
        let Some(desc) = lines.next_line().await? else {
            break;
        };
        columns.push((String::from(name), desc));

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                break;
            }
        }
    }

    Ok(columns)
}

async fn scrape(path: &Path, infos: &[(String, String)]) -> Result<Vec<Metric>, Error> {
    let query_fields = infos
        .iter()
        .map(|info| info.0.as_str())
        .collect::<Vec<_>>()
        .join(",");

    let mut cmd = tokio::process::Command::new(path);
    cmd.args(["--format", "csv", "--query-gpu", &query_fields])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .kill_on_drop(true);

    let mut child = cmd.spawn()?;

    let status = child.wait().await?;
    if !status.success() {
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| std::io::Error::other("Unable to take stderr of spawned process"))?;

        let mut output = String::new();
        stderr.read_to_string(&mut output).await?;

        return Err(Error::Exit(status, output));
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("Unable to take stdout of spawned process"))?;

    let mut lines = BufReader::new(stdout).lines();
    let mut columns = Vec::new();
    let mut name_index = None;
    let mut uuid_index = None;
    let mut vbios_version_index = None;
    let mut driver_version_index = None;
    let mut driver_model_current = None;
    let mut driver_model_pending = None;

    let mut metrics = Vec::new();
    while let Some(line) = lines.next_line().await? {
        if columns.is_empty() {
            columns = line
                .split(',')
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>();

            columns
                .iter()
                .enumerate()
                .for_each(|(index, column)| match column.as_str() {
                    "name" => name_index = Some(index),
                    "uuid" => uuid_index = Some(index),
                    "vbios_version" => vbios_version_index = Some(index),
                    "driver_version" => driver_version_index = Some(index),
                    "driver_model.current" => driver_model_current = Some(index),
                    "driver_model.pending" => driver_model_pending = Some(index),
                    _ => {}
                });

            continue;
        }

        let fields = line.split(',').map(|s| s.trim()).collect::<Vec<_>>();
        if fields.len() != columns.len() {
            // just in case
            continue;
        }

        let Some(uuid) = fields
            .get(uuid_index.unwrap())
            .map(|field| field.strip_prefix("GPU-").unwrap_or(field))
        else {
            continue;
        };

        {
            let mut tags = tags!(
                "uuid" => uuid
            );
            if let Some(Some(name)) = name_index.map(|index| fields.get(index)) {
                tags.insert("name", *name);
            }
            if let Some(Some(version)) = vbios_version_index.map(|index| fields.get(index)) {
                tags.insert("vbios_version", *version);
            }
            if let Some(Some(driver_version)) = driver_version_index.map(|index| fields.get(index))
            {
                tags.insert("driver_version", *driver_version);
            }
            if let Some(Some(driver_model_current)) =
                driver_model_current.map(|index| fields.get(index))
            {
                tags.insert("driver_model_current", *driver_model_current);
            }
            if let Some(Some(driver_model_pending)) =
                driver_model_pending.map(|index| fields.get(index))
            {
                tags.insert("driver_model_pending", *driver_model_pending);
            }

            metrics.push(Metric::gauge_with_tags(
                "nvidia_gpu_info",
                "A metric with a constant '1' value labeled by gpu uuid, name, driver_model_current, driver_model_pending, vbios_version, driver_version.",
                1,
                tags
            ));
        }

        for (index, (column, field)) in columns.iter().zip(fields.iter()).enumerate() {
            if *field == "[N/A]" {
                continue;
            }

            let mut multiplier = 1.0;
            let name = if let Some(stripped) = column.strip_suffix(" [W]") {
                format!("{}_watts", sanitize(stripped))
            } else if let Some(stripped) = column.strip_suffix(" [MHz]") {
                multiplier = 1_000_000.0;
                format!("{}_clock_hz", sanitize(stripped))
            } else if let Some(stripped) = column.strip_suffix(" [MiB]") {
                multiplier = 1_048_576.0;
                format!("{}_bytes", sanitize(stripped))
            } else if let Some(stripped) = column.strip_suffix(" [%]") {
                multiplier = 0.01;
                format!("{}_ratio", sanitize(stripped))
            } else if let Some(stripped) = column.strip_suffix(" [us]") {
                multiplier = 0.000001;
                format!("{}_seconds", sanitize(stripped))
            } else {
                sanitize(column)
            };

            let value = match *field {
                "Enabled" | "Yes" | "Active" => 1.0,
                "Disabled" | "No" | "Not Active" => 0.0,

                // "compute_mode"
                // The compute mode flag indicates whether individual or multiple compute applications may run on the GPU.
                "Default" => 0.0,
                "Exclusive_Thread" => 1.0,
                "Prohibited" => 2.0,
                "Exclusive_Process" => 3.0,

                // The current performance state for the GPU.
                // States range from P0 (maximum performance) to P12 (minimum performance).
                "P0" => 0.0,
                "P1" => 1.0,
                "P2" => 2.0,
                "P3" => 3.0,
                "P4" => 4.0,
                "P5" => 5.0,
                "P6" => 6.0,
                "P7" => 7.0,
                "P8" => 8.0,
                "P9" => 9.0,
                "P10" => 10.0,
                "P11" => 11.0,
                "P12" => 12.0,

                _ => {
                    let field = match field.split_once(' ') {
                        Some((field, _)) => field,
                        None => field,
                    };

                    match field.parse::<f64>() {
                        Ok(value) => value,
                        _ => {
                            let Some(stripped) = field.strip_prefix("0x") else {
                                continue;
                            };

                            let Ok(value) = u64::from_str_radix(stripped, 16) else {
                                continue;
                            };

                            value as f64
                        }
                    }
                }
            };

            metrics.push(Metric::gauge_with_tags(
                name,
                infos
                    .get(index)
                    .map(|info| Cow::<'static, str>::Owned(info.1.to_string()))
                    .unwrap_or(Cow::<'static, str>::Owned(column.to_string())),
                value * multiplier,
                tags!(
                    "uuid" => uuid,
                ),
            ))
        }
    }

    Ok(metrics)
}

fn sanitize(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    output.push_str("nvidia_");

    for c in input.chars() {
        if c.is_whitespace() {
            break;
        }

        if c.is_ascii_uppercase() {
            output.push('_');
            output.push(c.to_ascii_lowercase());
            continue;
        }

        if c == '.' {
            output.push('_');
            continue;
        }

        output.push(c);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }

    #[tokio::test]
    async fn lqf() {
        let metric_infos = load_query_field(Path::new("tests/nvidia/mock.sh"))
            .await
            .unwrap();
        for (name, desc) in metric_infos {
            println!("{name}: {desc}");
        }
    }
}
