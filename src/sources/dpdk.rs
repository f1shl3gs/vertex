use std::collections::BTreeMap;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use std::time::{Duration, Instant};

use configurable::configurable_component;
use event::{tags, Metric};
use framework::config::{default_interval, Output, SourceConfig, SourceContext};
use framework::Source;
use serde::Deserialize;
use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

fn default_socket_path() -> PathBuf {
    PathBuf::from("/var/run/dpdk/rte/dpdk_telemetry.v2")
}

#[configurable_component(source, name = "dpdk")]
struct Config {
    /// DPDK Telemetry path
    #[serde(default = "default_socket_path")]
    socket_path: PathBuf,

    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "dpdk")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let path = self.socket_path.clone();
        let mut shutdown = cx.shutdown;
        let mut output = cx.output;
        let mut ticker = tokio::time::interval(self.interval);

        Ok(Box::pin(async move {
            loop {
                tokio::select! {
                    _ = ticker.tick() => {},
                    _ = &mut shutdown => break,
                }

                let start = Instant::now();
                let result = gather(&path).await;
                let elapsed = start.elapsed();

                let collect_metrics = vec![
                    Metric::gauge("dpdk_up", "", result.is_ok()),
                    Metric::gauge("dpdk_scrape_duration_seconds", "", elapsed),
                ];

                let metrics = match result {
                    Ok(mut metrics) => {
                        metrics.extend(collect_metrics);
                        metrics
                    }
                    Err(err) => {
                        warn!(message = "gather metrics failed", ?err);

                        collect_metrics
                    }
                };

                if let Err(_err) = output.send(metrics).await {
                    break;
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::metrics()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

async fn gather(path: &PathBuf) -> std::io::Result<Vec<Metric>> {
    let mut stream = UnixSeqStream::connect(path)?;

    // read info
    let info = read::<Info>(&mut stream).await?;
    let mut metrics = vec![
        Metric::gauge_with_tags(
            "dpdk_info",
            "",
            1,
            tags!(
                "version" => &info.version,
            ),
        ),
        Metric::gauge("dpdk_pid", "", info.pid),
        Metric::gauge("dpdk_max_output_len", "", info.max_output_len),
    ];

    // get stats
    let heap_info = query::<HeapInfo>(&mut stream, "/eal/heap_info,0").await?;
    metrics.extend([
        Metric::gauge_with_tags(
            "dpdk_heap_id",
            "",
            heap_info.heap_id,
            tags!(
                "name" => &heap_info.name,
            ),
        ),
        Metric::gauge_with_tags(
            "dpdk_heap_size",
            "",
            heap_info.heap_size,
            tags!(
                "name" => &heap_info.name,
            ),
        ),
        Metric::gauge_with_tags(
            "dpdk_free_size",
            "",
            heap_info.free_size,
            tags!(
                "name" => &heap_info.name,
            ),
        ),
        Metric::gauge_with_tags(
            "dpdk_alloc_size",
            "",
            heap_info.alloc_size,
            tags!(
                "name" => &heap_info.name,
            ),
        ),
        Metric::gauge_with_tags(
            "dpdk_greatest_free_size",
            "",
            heap_info.greatest_free_size,
            tags!(
                "name" => &heap_info.name,
            ),
        ),
        Metric::gauge_with_tags(
            "dpdk_alloc_count",
            "",
            heap_info.alloc_count,
            tags!(
                "name" => &heap_info.name,
            ),
        ),
        Metric::gauge_with_tags(
            "dpdk_free_count",
            "",
            heap_info.free_count,
            tags!(
                "name" => heap_info.name,
            ),
        ),
    ]);

    let eth_devices = query::<Vec<i64>>(&mut stream, "/ethdev/list").await?;
    for id in eth_devices {
        let device = query::<EthDeviceInfo>(&mut stream, format!("/eth/device_info,{id}")).await?;

        let xstats = query::<BTreeMap<String, f64>>(&mut stream, "/ethdev/xstats,{id}").await?;
        for (key, value) in xstats {
            metrics.push(Metric::sum_with_tags(
                format!("dpdk_interface_{key}"),
                "DP-Service interface statistic",
                value,
                tags!(
                    "interface" => &device.name
                ),
            ));
        }
    }

    if let Some(nat_port_counts) =
        query::<Option<BTreeMap<String, i64>>>(&mut stream, "/dp_service/nat/used_port_count")
            .await?
    {
        for (key, value) in nat_port_counts {
            metrics.push(Metric::sum_with_tags(
                "dpdk_interface_nat_used_port_count",
                "DP-Service interface statistic",
                value,
                tags!(
                    "interface" => key
                ),
            ));
        }
    }

    if let Some(virt_svc_used_port_counts) =
        query::<Option<BTreeMap<String, i64>>>(&mut stream, "/dp_service/virtsvc/used_port_count")
            .await?
    {
        for (key, value) in virt_svc_used_port_counts {
            metrics.push(Metric::sum_with_tags(
                "dpdk_interface_virtsvc_used_port_count",
                "DP-Service interface statistic",
                value,
                tags!(
                    "interface" => key
                ),
            ));
        }
    }

    if let Some(call_count) =
        query::<Option<GraphCallCount>>(&mut stream, "/dp_service/graph/call_count").await?
    {
        for (key, value) in call_count.node_data {
            metrics.push(Metric::sum_with_tags(
                format!("dpdk_interface_call_count_{key}"),
                "",
                value,
                tags!(
                    "interface" => key
                ),
            ));
        }
    }

    Ok(metrics)
}

#[derive(Deserialize)]
struct Info {
    version: String,
    pid: i64,
    max_output_len: i64,
}

#[derive(Deserialize)]
struct HeapInfo {
    #[serde(rename = "Heap_id")]
    pub heap_id: i64,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Heap_size")]
    pub heap_size: i64,
    #[serde(rename = "Free_size")]
    pub free_size: i64,
    #[serde(rename = "Alloc_size")]
    pub alloc_size: i64,
    #[serde(rename = "Greatest_free_size")]
    pub greatest_free_size: i64,
    #[serde(rename = "Alloc_count")]
    pub alloc_count: i64,
    #[serde(rename = "Free_count")]
    pub free_count: i64,
}

#[derive(Deserialize)]
struct EthDeviceInfo {
    name: String,
}

#[derive(Deserialize)]
struct GraphCallCount {
    #[serde(rename = "Node_0_to_255")]
    node_data: BTreeMap<String, f64>,
}

struct UnixSeqStream {
    inner: AsyncFd<OwnedFd>,
}

impl UnixSeqStream {
    fn connect(path: impl AsRef<Path>) -> std::io::Result<Self> {
        use std::os::unix::ffi::OsStrExt;

        let path = path.as_ref().as_os_str().as_bytes();
        let fd = unsafe {
            let ret = libc::socket(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                0,
            );
            if ret == -1 {
                return Err(std::io::Error::last_os_error());
            }

            let mut sockaddr: libc::sockaddr_un = std::mem::zeroed();
            let max_len = size_of_val(&sockaddr.sun_path) - 1;

            if path.len() > max_len {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "path length execeeds maximum sockaddr length",
                ));
            }

            sockaddr.sun_family = libc::AF_UNIX as _;
            std::ptr::copy_nonoverlapping(
                path.as_ptr(),
                sockaddr.sun_path.as_mut_ptr() as *mut u8,
                path.len(),
            );
            sockaddr.sun_path[path.len()] = 0;

            let path_offset =
                sockaddr.sun_path.as_ptr() as usize - (&sockaddr as *const _ as usize);
            let addr_len = if cfg!(any(target_os = "linux", target_os = "android"))
                && path.first() == Some(&0)
            {
                path_offset + path.len()
            } else {
                path_offset + path.len() + 1
            };

            if libc::connect(
                ret,
                &sockaddr as *const _ as *const libc::sockaddr,
                addr_len as _,
            ) == -1
            {
                return Err(std::io::Error::last_os_error());
            }

            OwnedFd::from_raw_fd(ret)
        };

        Ok(UnixSeqStream {
            inner: AsyncFd::new(fd)?,
        })
    }
}

impl AsyncRead for UnixSeqStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        loop {
            let mut guard = ready!(self.inner.poll_read_ready(cx))?;

            let unfilled = buf.initialize_unfilled();
            match guard.try_io(|inner| {
                let ret = unsafe {
                    libc::recv(
                        inner.as_raw_fd(),
                        unfilled.as_mut_ptr() as *mut libc::c_void,
                        unfilled.len(),
                        0,
                    )
                };
                if ret == -1 {
                    return Err(std::io::Error::last_os_error());
                }

                Ok(ret as usize)
            }) {
                Ok(Ok(len)) => {
                    buf.advance(len);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(err)) => return Poll::Ready(Err(err)),
                Err(_would_block) => continue,
            }
        }
    }
}

impl AsyncWrite for UnixSeqStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        loop {
            let mut guard = ready!(self.inner.poll_write_ready(cx))?;

            match guard.try_io(|inner| {
                let ret = unsafe {
                    libc::send(
                        inner.as_raw_fd(),
                        buf.as_ptr() as *const libc::c_void,
                        buf.len(),
                        libc::MSG_NOSIGNAL,
                    )
                };
                if ret == -1 {
                    return Err(std::io::Error::last_os_error());
                }

                Ok(ret as usize)
            }) {
                Ok(res) => return Poll::Ready(res),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let ret = unsafe { libc::shutdown(self.inner.as_raw_fd(), libc::SHUT_RDWR) };
        if ret == -1 {
            return Poll::Ready(Err(std::io::Error::last_os_error()));
        }

        Poll::Ready(Ok(()))
    }
}

async fn query<T: for<'a> Deserialize<'a>>(
    stream: &mut UnixSeqStream,
    command: impl AsRef<[u8]>,
) -> std::io::Result<T> {
    stream.write_all(command.as_ref()).await?;

    let mut resp = read::<BTreeMap<String, T>>(stream).await?;

    match resp.pop_first() {
        Some((_key, value)) => Ok(value),
        None => Err(std::io::ErrorKind::NotFound.into()),
    }
}

async fn read<T: for<'a> Deserialize<'a>>(stream: &mut UnixSeqStream) -> std::io::Result<T> {
    let mut buf = [0u8; 1024];
    let size = stream.read(&mut buf).await?;

    serde_json::from_slice(&buf[0..size])
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
