mod built_info;
#[cfg(feature = "jemalloc")]
mod jemalloc;
#[cfg(target_os = "linux")]
mod linux;
mod runtime;

use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::time::Duration;

use configurable::configurable_component;
use event::Metric;
use framework::Source;
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;

#[cfg(target_os = "linux")]
fn default_proc_path() -> PathBuf {
    PathBuf::from("/proc")
}

#[configurable_component(source, name = "selfstat")]
struct Config {
    /// The path of `/proc`
    #[cfg(target_os = "linux")]
    #[serde(default = "default_proc_path")]
    proc_path: PathBuf,

    /// The interval between scrapes.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "selfstat")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        Ok(Box::pin(run(
            self.proc_path.clone(),
            self.interval,
            cx.shutdown,
            cx.output,
        )))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

async fn run(
    root: PathBuf,
    interval: Duration,
    mut shutdown: ShutdownSignal,
    mut output: Pipeline,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        match gather(&root).await {
            Ok(mut metrics) => {
                let now = Some(chrono::Utc::now());
                metrics.iter_mut().for_each(|m| m.timestamp = now);

                if let Err(err) = output.send(metrics).await {
                    error!(
                        message = "Error sending selfstat metrics",
                        %err
                    );

                    return Err(());
                }
            }
            Err(err) => {
                warn!(
                    message = "gather selfstat failed",
                    %err
                );
            }
        }
    }

    Ok(())
}

async fn gather(root: &Path) -> Result<Vec<Metric>, std::io::Error> {
    #[cfg(target_os = "linux")]
    let mut metrics = linux::proc_info(root);

    #[cfg(not(target_os = "linux"))]
    let mut metrics = vec![];

    metrics.push(built_info::built_info());

    metrics.extend(runtime::metrics());

    #[cfg(feature = "jemalloc")]
    metrics.extend(jemalloc::alloc_metrics());

    #[cfg(feature = "tracked_allocator")]
    {
        let (alloc, allocated, dealloc, deallocated) =
            crate::common::tracked_allocator::statistics();

        metrics.extend([
            Metric::sum("process_alloc_total", "", alloc),
            Metric::sum("process_allocated_bytes", "", allocated),
            Metric::sum("process_dealloc_total", "", dealloc),
            Metric::sum("process_deallocated_bytes", "", deallocated),
        ])
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
