use crate::{
    tags,
    gauge_metric,
    sum_metric,
    config::{
        SourceConfig, SourceContext, DataType,
        deserialize_duration, serialize_duration,
    },
    sources::{
        Source,
        node::read_to_string,
    },
    event::{Metric, MetricValue},
    shutdown::ShutdownSignal,
    pipeline::Pipeline,
    event::Event,
};
use std::{
    fmt::Debug,
    io::Read,
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::IntervalStream;
use futures::{StreamExt, SinkExt};
use tokio::sync::RwLock;

const USER_HZ: f64 = 100.0;

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
struct SelfStatConfig {
    #[serde(default = "default_interval")]
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    interval: chrono::Duration,
}

fn default_interval() -> chrono::Duration {
    chrono::Duration::seconds(15)
}

#[async_trait::async_trait]
#[typetag::serde(name = "selfstat")]
impl SourceConfig for SelfStatConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let ss = SelfStat::from(self);

        Ok(Box::pin(ss.run(ctx.shutdown, ctx.out)))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "selfstat"
    }
}


struct SelfStat {
    interval: std::time::Duration,
    cpu_total: RwLock<f64>,
}


impl From<&SelfStatConfig> for SelfStat {
    fn from(conf: &SelfStatConfig) -> Self {
        Self {
            interval: conf.interval.to_std().unwrap(),
            cpu_total: RwLock::new(0.0),
        }
    }
}

impl SelfStat {
    async fn run(self, shutdown: ShutdownSignal, mut out: Pipeline) -> Result<(), ()> {
        let interval = tokio::time::interval(self.interval);
        let mut ticker = IntervalStream::new(interval)
            .take_until(shutdown);

        while ticker.next().await.is_some() {
            match gather().await {
                Ok(metrics) => {
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                    let now = now.as_millis() as i64;

                    let mut stream = futures::stream::iter(metrics)
                        .map(|mut m| {
                            m.timestamp = now;
                            Event::Metric(m)
                        })
                        .map(Ok);

                    out.send_all(&mut stream).await;
                }
                Err(err) => {
                    warn!("gather selfstat failed"; "err" => err);
                }
            }
        }

        Ok(())
    }
}

async fn gather() -> Result<Vec<Metric>, std::io::Error> {
    let pid = unsafe { libc::getpid() as i32 };
    let fds = open_fds(pid)? as f64;
    let max_fds = max_fds(pid)? as f64;
    let (cpu_total, threads, start_time, vsize, rss) = get_proc_stat("/proc", pid).await?;

    let page_size = 4096.0;

    Ok(vec![
        gauge_metric! {
            "process_max_fds",
            "Maximum number of open file descriptors.",
            max_fds,
        },
        gauge_metric!(
            "process_open_fds",
            "Number of open file descriptors",
            fds,
        ),
        sum_metric!(
            "process_cpu_seconds_total",
            "Total user and system CPU time spent in seconds",
            cpu_total,
        ),
        sum_metric!(
            "process_start_time_seconds",
            "Start time of the process since unix epoch in seconds",
            start_time,
        ),
        gauge_metric!(
            "process_virtual_memory_bytes",
            "Virtual memory size in bytes",
            vsize,
        ),
        gauge_metric!(
            "process_resident_memory_bytes",
            "Resident memory size in bytes",
            rss * page_size,
        ),
        gauge_metric!(
            "process_threads",
            "Number of OS threads created",
            threads,
        ),
    ])
}

fn open_fds(pid: i32) -> Result<usize, std::io::Error> {
    let path = format!("/proc/{}/fd", pid);
    std::fs::read_dir(path)?.fold(Ok(0), |acc, i| {
        let mut acc = acc?;
        let ty = i?.file_type()?;
        if !ty.is_dir() {
            acc += 1;
        }

        Ok(acc)
    })
}

fn find_statistic(all: &str, pat: &str) -> Result<f64, std::io::Error> {
    if let Some(idx) = all.find(pat) {
        let mut iter = (all[idx + pat.len()..]).split_whitespace();
        if let Some(v) = iter.next() {
            return v
                .parse()
                .map_err(|e| {
                    // Error::Msg(format!("read statistic {} failed: {}", pat, e))
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, e)
                });
        }
    }

    // Err(Error::Msg(format!("read statistic {} failed", pat)))
    Err(std::io::Error::from(std::io::ErrorKind::InvalidInput))
}

const MAXFD_PATTERN: &str = "Max open files";

fn max_fds(pid: i32) -> Result<f64, std::io::Error> {
    let mut buffer = String::new();
    std::fs::File::open(&format!("/proc/{}/limits", pid))
        .and_then(|mut f| f.read_to_string(&mut buffer))?;

    find_statistic(&buffer, MAXFD_PATTERN)
}

async fn get_proc_stat(root: &str, pid: i32) -> Result<(f64, f64, f64, f64, f64), std::io::Error> {
    let path = format!("{}/{}/stat", root, pid);
    let content = read_to_string(&path).await?;
    let parts = content.split_ascii_whitespace()
        .collect::<Vec<_>>();

    let utime = parts[13].parse().unwrap_or(0f64);
    let stime = parts[14].parse().unwrap_or(0f64);
    let threads = parts[19].parse().unwrap_or(0f64);
    let start_time = parts[21].parse().unwrap_or(0f64);
    let vsize = parts[22].parse().unwrap_or(0f64);
    let rss = parts[23].parse().unwrap_or(0f64);

    // TODO: fix start time
    Ok(((utime + stime) / USER_HZ, threads, (start_time) / USER_HZ, vsize, rss))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proc_stat() {
        let (cpu_time, threads, _, vsize, rss) = get_proc_stat("testdata/node_metrics/fixtures/proc", 26231).await.unwrap();

        assert_eq!(cpu_time, 17.21);
        assert_eq!(threads, 1.0);
        assert_eq!(vsize, 56274944.0);
        assert_eq!(rss, 1981.0);
    }
}