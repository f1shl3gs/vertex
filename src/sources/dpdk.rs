use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll, ready};
use std::time::{Duration, Instant};

use configurable::configurable_component;
use event::{Metric, tags};
use framework::Source;
use framework::config::{Output, SourceConfig, SourceContext, default_interval};
use serde::Deserialize;
use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

fn default_socket_path() -> PathBuf {
    PathBuf::from("/var/run/dpdk/rte/dpdk_telemetry.v2")
}

#[configurable_component(source, name = "dpdk")]
struct Config {
    /// DPDK Telemetry path, vertex might need privilege to access this UnixSeqPacket
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
        Metric::gauge("dpdk_process_pid", "", info.pid),
        Metric::gauge("dpdk_max_output_len", "", info.max_output_len),
    ];

    metrics.extend(cpu(&mut stream).await?);
    metrics.extend(memory(&mut stream).await?);
    metrics.extend(ethdev(&mut stream).await?);

    Ok(metrics)
}

#[derive(Deserialize)]
struct Info {
    version: String,
    pid: i64,
    max_output_len: i64,
}

#[derive(Deserialize)]
struct Cycles {
    total_cycles: i64,
    busy_cycles: i64,
}

#[derive(Deserialize)]
struct LCoreInfo {
    socket: i64,
    role: String,
    cpuset: Vec<i64>,

    // those two fields only available when `record-core-cycles` enabled
    #[serde(flatten)]
    cycles: Option<Cycles>,
}

async fn cpu(stream: &mut UnixSeqStream) -> std::io::Result<Vec<Metric>> {
    let ids = query::<Vec<i64>>(stream, "/eal/lcore/list").await?;

    let mut metrics = Vec::with_capacity(ids.len() * 2);
    for id in ids {
        let info = query::<LCoreInfo>(stream, format!("/eal/lcore/info,{id}")).await?;
        if info.cpuset.is_empty() {
            continue;
        }

        if let Some(cycle) = info.cycles {
            let cpu = info
                .cpuset
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",");
            metrics.extend([
                Metric::sum_with_tags(
                    "dpdk_cpu_total_cycles",
                    "Total number of CPU cycles",
                    cycle.total_cycles,
                    tags!(
                        "numa" => info.socket,
                        "cpu" => cpu.clone(),
                        "role" => info.role.clone(),
                    ),
                ),
                Metric::sum_with_tags(
                    "dpdk_cpu_busy_cycles",
                    "Number of busy CPU cycles",
                    cycle.busy_cycles,
                    tags!(
                        "numa" => info.socket,
                        "cpu" => cpu,
                        "role" => info.role,
                    ),
                ),
            ])
        }
    }

    Ok(metrics)
}

#[derive(Deserialize)]
struct MemZone {
    #[serde(rename = "Zone")]
    zone: i64,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Length")]
    length: i64,
    #[serde(rename = "Socket")]
    socket: i64,
    #[serde(rename = "Flags")]
    flags: i64,
    #[serde(rename = "Hugepage_size")]
    hugepage_size: i64,
    #[serde(rename = "Hugepage_base")]
    hugepage_base: String,
    #[serde(rename = "Hugepage_used")]
    hugepage_used: i64,
}

async fn memory(stream: &mut UnixSeqStream) -> std::io::Result<Vec<Metric>> {
    let ids = query::<Vec<i64>>(stream, "/eal/memzone_list").await?;

    let mut used = 0;
    let mut zones = BTreeMap::new();

    let mut metrics = Vec::with_capacity(ids.len() + 2);
    for id in ids {
        let zone = query::<MemZone>(stream, format!("/eal/memzone_info,{id}")).await?;
        metrics.push(Metric::gauge_with_tags(
            "dpdk_memzone_info",
            "DPDK memzone information",
            1,
            tags!(
                "zone" => zone.zone,
                "name" => zone.name,
                "socket" => zone.socket,
                "flags" => zone.flags,
            ),
        ));

        let start = i64::from_str_radix(&zone.hugepage_base[2..], 16)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        let end = start + zone.hugepage_size * zone.hugepage_used;
        used += zone.length;

        if let ControlFlow::Continue(_) = zones.clone().into_iter().try_for_each(|(s, e)| {
            if s < start && start < e && e < end {
                zones.insert(s, end);
                return ControlFlow::Break(());
            }

            if start < s && s < end && end < e {
                zones.remove(&s);
                zones.insert(start, e);
                return ControlFlow::Break(());
            }

            ControlFlow::Continue(())
        }) {
            zones.insert(start, end);
        }
    }

    metrics.extend([
        Metric::gauge(
            "dpdk_memory_total_bytes",
            "The total size of reserved memory in bytes.",
            zones
                .into_iter()
                .map(|(start, end)| end - start)
                .sum::<i64>(),
        ),
        Metric::gauge(
            "dpdk_memory_used_bytes",
            "The currently used memory in bytes",
            used,
        ),
    ]);

    Ok(metrics)
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct EthDeviceInfo {
    name: String,
    state: i64,
    nb_rx_queues: i64,
    nb_tx_queues: i64,
    port_id: i64,
    mtu: i64,
    rx_mbuf_size_min: i64,
    mac_addr: String,
    promiscuous: i64,
    scattered_rx: i64,
    all_multicast: i64,
    dev_started: i64,
    lro: i64,
    dev_configured: i64,
    rxq_state: Vec<i64>,
    txq_state: Vec<i64>,
    numa_node: i64,
    dev_flags: String,
    ethdev_rss_hf: String,
}

#[derive(Deserialize)]
struct EthDeviceStats {
    ipackets: i64,
    opackets: i64,
    ibytes: i64,
    obytes: i64,
    imissed: i64,
    ierrors: i64,
    oerrors: i64,
    rx_nombuf: i64,
}

async fn ethdev(stream: &mut UnixSeqStream) -> std::io::Result<Vec<Metric>> {
    let ids = query::<Vec<i64>>(stream, "/ethdev/list").await?;

    let mut metrics = Vec::with_capacity(ids.len());
    for id in ids {
        let info = query::<EthDeviceInfo>(stream, format!("/ethdev/info,{id}")).await?;
        let stats = query::<EthDeviceStats>(stream, format!("/ethdev/stats,{id}")).await?;

        metrics.extend([
            Metric::gauge_with_tags(
                "dpdk_eth_device_info",
                "Ethernet device info",
                1,
                tags!(
                    "port" => &info.name,
                    "mtu" => info.mtu,
                ),
            ),
            Metric::sum_with_tags(
                "dpdk_eth_device_receive_packets",
                "Number of successfully received packets.",
                stats.ipackets,
                tags!(
                    "port" => &info.name
                ),
            ),
            Metric::sum_with_tags(
                "dpdk_eth_device_transmit_packets",
                "Number of successfully transmitted packets.",
                stats.opackets,
                tags!(
                    "port" => &info.name
                ),
            ),
            Metric::sum_with_tags(
                "dpdk_eth_device_receive_bytes",
                "Number of successfully received bytes.",
                stats.ibytes,
                tags!(
                    "port" => &info.name
                ),
            ),
            Metric::sum_with_tags(
                "dpdk_eth_device_transmit_bytes",
                "Number of successfully transmitted bytes.",
                stats.obytes,
                tags!(
                    "port" => &info.name
                ),
            ),
            Metric::sum_with_tags(
                "dpdk_eth_device_receive_missed_packets",
                "Number of packets dropped by the HW because Rx queues are full.",
                stats.imissed,
                tags!(
                    "port" => &info.name
                ),
            ),
            Metric::sum_with_tags(
                "dpdk_eth_device_receive_errors",
                "Number of erroneous received packets.",
                stats.ierrors,
                tags!(
                    "port" => &info.name
                ),
            ),
            Metric::sum_with_tags(
                "dpdk_eth_device_transmit_errors",
                "Number of packet transmission failures.",
                stats.oerrors,
                tags!(
                    "port" => &info.name
                ),
            ),
            Metric::sum_with_tags(
                "dpdk_eth_device_receive_nombuf",
                "Number of Rx mbuf allocation failures.",
                stats.rx_nombuf,
                tags!(
                    "port" => info.name
                ),
            ),
        ])
    }

    Ok(metrics)
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
    let mut buf = [0u8; 16 * 1024];

    // Unix SeqPacket is something like UDP
    let size = stream.read(&mut buf).await?;

    serde_json::from_slice(&buf[..size])
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
