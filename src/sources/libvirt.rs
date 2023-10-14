use std::borrow::Cow;
use std::time::{Duration, Instant};

use configurable::configurable_component;
use event::tags::Key;
use event::{tags, Metric};
use framework::config::{default_interval, DataType, Output, SourceConfig, SourceContext};
use framework::Source;
use virt::{Client, Error};

const DOMAIN_KEY: Key = Key::from_static("domain");

fn default_sock() -> String {
    "/run/libvirt/libvirt-sock-ro".to_string()
}

/// This source connects to libvirt daemon and collect per-domain metrics related
/// to CPU, memory, disk and network usage.
#[configurable_component(source, name = "libvirt")]
struct Config {
    /// The socket path of libvirtd, read permission is required.
    #[serde(default = "default_sock")]
    #[configurable(required)]
    sock: String,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "libvirt")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let sock = self.sock.clone();
        let mut ticker = tokio::time::interval(self.interval);
        let SourceContext {
            mut output,
            mut shutdown,
            ..
        } = cx;

        Ok(Box::pin(async move {
            loop {
                tokio::select! {
                    biased;

                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                let start = Instant::now();
                let result = gather_v2(&sock).await.map_err(|err| {
                    warn!(message = "Scrape libvirt metrics failed", ?err);
                    err
                });
                let elapsed = start.elapsed().as_secs_f64();
                let up = result.is_ok();

                let mut metrics = result.unwrap_or_default();
                metrics.extend_from_slice(&[
                    Metric::gauge(
                        "libvirt_up",
                        "Whether scraping libvirt's metrics was successful",
                        up,
                    ),
                    Metric::gauge("libvirt_scrape_duration_seconds", "", elapsed),
                ]);

                let timestamp = Some(chrono::Utc::now());
                metrics.iter_mut().for_each(|m| m.timestamp = timestamp);
                if let Err(err) = output.send(metrics).await {
                    error!(
                        message = "Error sending libvirt metrics",
                        %err
                    );

                    return Err(());
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }
}

async fn gather_v2(path: &str) -> Result<Vec<Metric>, Error> {
    let mut cli = Client::connect(path).await?;
    cli.open().await?;

    let hyper_version = cli.hyper_version().await?;
    let libvirtd_version = cli.version().await?;

    let mut metrics = vec![Metric::gauge_with_tags(
        "libvirt_version_info",
        "Versions of virtualization components",
        1,
        tags!(
            "hypervisor_running" => hyper_version,
            "libvirtd_running" => libvirtd_version,
        ),
    )];

    // Collect domain metrics
    for stat in cli.get_all_domain_stats().await? {
        let dom = stat.domain();
        let info = cli.get_domain_info(dom).await?;
        let uuid = dom.uuid();
        let name = &dom.name().to_string();

        // Report domain info
        let xml = cli.domain_xml(dom).await?;
        let schema::Domain { devices, metadata } = quick_xml::de::from_str::<schema::Domain>(&xml)
            .map_err(|err| Error::IO(std::io::Error::new(std::io::ErrorKind::InvalidData, err)))?;

        metrics.extend_from_slice(&[
            Metric::gauge_with_tags(
                "libvirt_domain_info_meta",
                "Domain metadata",
                1,
                tags!(
                    DOMAIN_KEY => name,
                    "uuid" => uuid,
                    "instance_name" => metadata.instance.name,
                    "flavor" => metadata.instance.flavor.name,
                    "user_name" => metadata.instance.owner.user.name,
                    "user_uuid" => metadata.instance.owner.user.uuid,
                    "project_name" => metadata.instance.owner.project.name,
                    "project_uuid" => metadata.instance.owner.project.uuid,
                    "root_type" => metadata.instance.root.typ,
                    "root_uuid" => metadata.instance.root.uuid
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_info_maximum_memory_bytes",
                "Maximum allowed memory of the domain, in bytes",
                info.max_mem * 1024,
                tags!(
                    DOMAIN_KEY => name
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_info_memory_usage_bytes",
                "Memory usage of the domain, in bytes",
                info.memory * 1024,
                tags!(
                    DOMAIN_KEY => name
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_info_virtual_cpus",
                "Number of virtual CPUs for the domain",
                info.nr_virt_cpu,
                tags!(
                    DOMAIN_KEY => name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_info_cpu_time_seconds_total",
                "Amount of CPU time used by the domain, in seconds",
                info.cpu_time as f64 / 1000.0 / 1000.0 / 1000.0, // From ns to s
                tags!(
                    DOMAIN_KEY => name,
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_info_vstate",
                "Virtual domain state. 0: no, 1: running, 2: blocked, 3: paused, 4: shutdown, 5: shutoff, 6: crashed, 7: suspended",
                info.state as u32,
                tags!(
                    DOMAIN_KEY => name,
                ),
            ),
        ]);

        // Report vcpus
        match cli.get_domain_vcpus(dom, info.nr_virt_cpu as i32).await {
            Ok(vcpus) => {
                for vcpu in vcpus {
                    let vcpu_num = vcpu.number.to_string();
                    let (delay, wait) = stat.vcpu_delay_and_wait(vcpu.number);

                    metrics.extend_from_slice(&[
                        Metric::gauge_with_tags(
                            "libvirt_domain_vcpu_state",
                            "VCPU state. 0: offline, 1: running, 2: blocked",
                            vcpu.state,
                            tags!(
                                DOMAIN_KEY => name,
                                "vcpu" => &vcpu_num,
                            ),
                        ),
                        Metric::sum_with_tags(
                            "libvirt_domain_vcpu_time_seconds_total",
                            "Amount of CPU time used by the domain's VCPU, in seconds",
                            vcpu.cpu_time as f64 / 1000.0 / 1000.0 / 1000.0, // From ns to s
                            tags!(
                                DOMAIN_KEY => name,
                                "vcpu" => &vcpu_num
                            ),
                        ),
                        Metric::gauge_with_tags(
                            "libvirt_domain_vcpu_cpu",
                            "Real CPU number, or one of the values from virVcpuHostCpuState",
                            vcpu.cpu,
                            tags!(
                                DOMAIN_KEY => name,
                                "vcpu" => &vcpu_num
                            ),
                        ),
                        Metric::sum_with_tags(
                            "libvirt_domain_vcpu_wait_seconds_total",
                            "Vcpu's wait_sum metrics. CONFIG_SCHEDSTATS has to be enabled",
                            wait as f64 / 1000.0 / 1000.0 / 1000.0,
                            tags!(
                                DOMAIN_KEY => name,
                                "vcpu" => &vcpu_num,
                            ),
                        ),
                        Metric::sum_with_tags(
                            "libvirt_domain_vcpu_delay_seconds_total",
                            "Amount of CPU time used by the domain's VCPU, in seconds.\
                Vcpu's delay metric. Time the vcpu thread was enqueued by the host \
                scheduler, but was waiting in the queue instead of running. Exposed to \
                the VM as a steal time",
                            delay as f64 / 1000.0 / 1000.0 / 1000.0,
                            tags!(
                                DOMAIN_KEY => name,
                                "vcpu" => &vcpu_num
                            ),
                        ),
                    ]);
                }
            }
            Err(err) => {
                match err {
                    Error::Libvirt(lerr) => {
                        // See also. https://libvirt.org/html/libvirt-virterror.html#virErrorNumber
                        if lerr.code != 55 {
                            // VIR_ERR_OPERATION_INVALID
                            return Err(Error::Libvirt(lerr));
                        }
                    }
                    _ => return Err(err),
                }
            }
        }

        // Report block device statistics
        for block in stat.blocks() {
            // Ugly hack to avoid getting metrics from cdrom block device
            // TODO: somehow check the disk 'device' field for 'cdrom' string
            if block.name == "hdc" || block.name == "hda" {
                continue;
            }

            let dev = devices
                .disks
                .iter()
                .find(|dev| dev.target.dev == block.name)
                .cloned()
                .unwrap_or_default();

            let disk_source = if !block.path.is_empty() {
                &block.path
            } else {
                &dev.source.name
            };

            metrics.extend_from_slice(&[
                Metric::gauge_with_tags(
                    "libvirt_domain_block_meta",
                    "Block device metadata info. Device name, source file, serial.",
                    1,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &dev.target.dev,
                        "source_file" => disk_source,
                        "serial" => &dev.serial,
                        "bus" => &dev.target.bus,
                        "disk_type" => &dev.disk_type,
                        "driver_type" => &dev.driver.typ,
                        "cache" => &dev.driver.cache,
                        "discard" => &dev.driver.discard
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_block_stats_read_bytes_total",
                    "Number of bytes read from a block device, in bytes.",
                    block.read_bytes,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_block_stats_read_requests_total",
                    "Number of read requests from a block device",
                    block.read_requests,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_block_stats_read_time_seconds_total",
                    "Total time spent on reads from a block device, in seconds",
                    block.read_time as f64 / 1000.0 / 1000.0 / 1000.0, // From ns to s
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_block_stats_write_bytes_total",
                    "Number of bytes written to a block device, in bytes",
                    block.write_bytes,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_block_stats_write_requests_total",
                    "Number of write requests to a block device",
                    block.write_requests,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_block_stats_write_time_seconds_total",
                    "Total time spent on writes on a block device, in seconds",
                    block.write_time as f64 / 1000.0 / 1000.0 / 1000.0, // From ns to s
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_block_stats_flush_requests_total",
                    "Total flush requests from a block device",
                    block.flush_requests,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_block_stats_flush_time_seconds_total",
                    "Total time in seconds spent on cache flushing to a block device",
                    block.flush_time as f64 / 1000.0 / 1000.0 / 1000.0, // From ns to s
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_allocation",
                    "Offset of the highest written sector on a block device",
                    block.allocation,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_capacity_bytes",
                    "Logical size in bytes of the block device backing image",
                    block.capacity,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_physical_bytes",
                    "Physical size in bytes of the container of the backing image",
                    block.physical,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &block.name,
                    ),
                ),
            ]);

            match cli.block_io_tune(dom, &block.name).await {
                Err(err) => match err {
                    Error::Libvirt(ref lerr) => {
                        // See also: https://github.com/libvirt/libvirt/blob/56fbabf1a1e272c6cc50adcb603996cf8e94ad08/include/libvirt/virterror.h#L209
                        match lerr.code {
                            55 => {
                                warn!(message = "Invalid operation block_io_tune", ?err)
                            }
                            84 => {
                                warn!(message = "Unsupported operation block_io_tune", ?err)
                            }
                            _ => return Err(err),
                        }
                    }
                    _ => return Err(err),
                },
                Ok(params) => metrics.extend_from_slice(&[
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_total_bytes",
                        "Total throughput limit in bytes per second",
                        params.total_bytes_sec,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_read_bytes",
                        "Read throughput limit in bytes per second",
                        params.read_bytes_sec,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_write_bytes",
                        "Write throughput limit in bytes per second",
                        params.write_bytes_sec,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_total_requests",
                        "Total requests per second limit",
                        params.total_iops_sec,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_read_requests",
                        "Read requests per second limit",
                        params.read_iops_sec,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_write_requests",
                        "Write requests per second limit",
                        params.write_iops_sec,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_burst_total_bytes",
                        "Total throughput burst limit in bytes per second",
                        params.total_bytes_sec_max,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_burst_read_bytes",
                        "Read throughput burst limit in bytes per second",
                        params.read_bytes_sec_max,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_burst_write_bytes",
                        "Write throughput burst limit in bytes per second",
                        params.write_bytes_sec_max,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_burst_total_requests",
                        "Total requests per second burst limit",
                        params.total_iops_sec_max,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_burst_total_bytes_length_seconds",
                        "Total throughput burst time in seconds",
                        params.total_bytes_sec_max_length,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_burst_read_bytes_length_seconds",
                        "Read throughput burst time in seconds",
                        params.read_bytes_sec_max_length,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_burst_write_bytes_length_seconds",
                        "Write throughput burst time in seconds",
                        params.write_bytes_sec_max_length,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_burst_length_total_requests_seconds",
                        "Total requests per second burst time in seconds",
                        params.total_iops_sec_max_length,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_burst_length_read_requests_seconds",
                        "Read requests per second burst time in seconds",
                        params.read_iops_sec_max_length,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_limit_burst_length_write_requests_seconds",
                        "Write requests per second burst time in seconds",
                        params.write_bytes_sec_max_length,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "libvirt_domain_block_stats_size_iops_bytes",
                        "The size of IO operations per seconds permitted through a block device",
                        params.size_iops_sec,
                        tags!(
                            DOMAIN_KEY => name,
                            "target_device" => &block.name,
                        ),
                    ),
                ]),
            }
        }

        // Report network interface statistics
        for iface in stat.networks() {
            let (source_bridge, virtual_interface) = devices
                .interfaces
                .iter()
                .find(|net| net.target.dev == iface.name)
                .map_or((Cow::from(""), Cow::from("")), |net| {
                    (
                        Cow::from(net.source.bridge.clone()),
                        Cow::from(net.virtual_port.parameters.interface_id.clone()),
                    )
                });

            if !source_bridge.is_empty() || !virtual_interface.is_empty() {
                metrics.push(Metric::gauge_with_tags(
                    "libvirt_domain_interface_meta",
                    "Interfaces metadata. Source bridge, target device, interface uuid",
                    1,
                    tags!(
                        DOMAIN_KEY => name,
                        "source_bridge" => source_bridge,
                        "target_device" => &iface.name,
                        "virtual_interface" => virtual_interface,
                    ),
                ));
            }

            metrics.extend_from_slice(&[
                Metric::sum_with_tags(
                    "libvirt_domain_interface_stats_receive_bytes_total",
                    "Number of bytes received on a network interface, in bytes",
                    iface.rx_bytes,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &iface.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_interface_stats_receive_packets_total",
                    "Number of packets received on a network interface",
                    iface.rx_packets,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &iface.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_interface_stats_receive_errors_total",
                    "Number of packet receive errors on a network interface",
                    iface.rx_errs,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &iface.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_interface_stats_receive_drop_total",
                    "Number of packets receive drops on a network interface",
                    iface.rx_bytes,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &iface.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_interface_stats_transmit_bytes_total",
                    "Number of bytes transmitted on a network interface, in bytes",
                    iface.tx_bytes,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &iface.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_interface_stats_transmit_packets_total",
                    "Number of packets transmitted on a network interface",
                    iface.tx_packets,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &iface.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_interface_stats_transmit_errors_total",
                    "Number of packet transmit errors on a network interface",
                    iface.tx_errs,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &iface.name,
                    ),
                ),
                Metric::sum_with_tags(
                    "libvirt_domain_interface_stats_transmit_drop_total",
                    "Number of packets transmit drops on a network interface",
                    iface.tx_bytes,
                    tags!(
                        DOMAIN_KEY => name,
                        "target_device" => &iface.name,
                    ),
                ),
            ])
        }

        // Collect Memory Stats
        let stats = cli.domain_memory_stats(dom, 11, 0).await?;
        let used_percent = if stats.usable != 0 && stats.available != 0 {
            (stats.available - stats.usable) as f64 / stats.available as f64 / 100.0
        } else {
            0.0
        };

        metrics.extend_from_slice(&[
            Metric::sum_with_tags(
                "libvirt_domain_memory_stats_major_fault_total",
                "Page faults occur when a process makes a valid access to virtual memory that is not available.\
            When servicing the page fault, if disk IO is required, it is considered a major fault",
                stats.major_fault,
                tags!(
                    DOMAIN_KEY => name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_memory_stats_minor_fault_total",
                "Page faults occur when a process makes a valid access to virtual memory that is not available.\
             When servicing the page not fault, if disk IO is required, it is considered a minor fault.",
                stats.minor_fault,
                tags!(
                    DOMAIN_KEY => name,
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_memory_stats_unused_bytes",
                "The amount of memory left completely unused by the system. Memory that is \
            available but used for reclaimable cache should NOT be reported as free. This value \
            is expressed in bytes",
                stats.unused * 1024,
                tags!(
                    DOMAIN_KEY => name,
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_memory_stats_available_bytes",
                "The total amount of usable memory as seen by the domain. This value may be less \
            than the amount of memory assigned to the domain if a balloon driver is in use or if \
            the guest OS does not initialize all assigned pages. This value is expressed in bytes",
                stats.available * 1024,
                tags!(
                    DOMAIN_KEY => name,
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_memory_stats_actual_balloon_bytes",
                "Current balloon value (in bytes)",
                stats.actual_balloon * 1024,
                tags!(
                    DOMAIN_KEY => name,
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_memory_stats_rss_bytes",
                "Resident Set Size of the process running the domain. This value is in bytes",
                stats.rss * 1024,
                tags!(
                    DOMAIN_KEY => name,
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_memory_stats_usable_bytes",
                "How much the balloon can be inflated without pushing the guest system to swap, \
            corresponds to 'Available' in /proc/meminfo",
                stats.usable * 1024,
                tags!(
                    DOMAIN_KEY => name,
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_memory_stats_disk_cache_bytes",
                "The amount of memory, that can be quickly reclaimed without additional I/O \
            (in bytes). Typically these pages are used for caching files from disk",
                stats.disk_caches * 1024,
                tags!(
                    DOMAIN_KEY => name,
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_memory_stats_used_percent",
                "The amount of memory in percent, that used by domain",
                used_percent,
                tags!(
                    DOMAIN_KEY => name,
                ),
            )
        ]);
    }

    // Collect storage pool info
    let pools = cli.storage_pools().await?;
    for info in pools {
        let name = info.name;
        metrics.extend_from_slice(&[
            Metric::gauge_with_tags(
                "libvirt_pool_info_capacity_bytes",
                "Pool capacity, in bytes",
                info.capacity,
                tags!(
                    "pool" => &name,
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_pool_info_allocation_bytes",
                "Pool allocation, in bytes",
                info.allocation,
                tags!(
                    "pool" => &name,
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_pool_info_available_bytes",
                "Pool available, in bytes",
                info.available,
                tags!(
                    "pool" => &name,
                ),
            ),
        ]);
    }

    Ok(metrics)
}

mod schema {
    use serde::Deserialize;

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct DiskTarget {
        pub dev: String,
        pub bus: String,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct DiskSource {
        pub file: String,
        #[serde(default)]
        pub name: String,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct DiskDriver {
        #[serde(rename = "type")]
        pub typ: String,
        #[serde(default)]
        pub cache: String,
        #[serde(default)]
        pub discard: String,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct Disk {
        pub device: String,
        #[serde(rename = "type")]
        pub disk_type: String,
        pub target: DiskTarget,
        pub source: DiskSource,
        #[serde(default)]
        pub serial: String,
        pub driver: DiskDriver,
    }

    #[derive(Debug, Deserialize)]
    pub struct InterfaceSource {
        #[serde(default)]
        pub bridge: String,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct InterfaceTarget {
        pub dev: String,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct InterfaceVirtualPortParam {
        #[serde(rename = "interfaceid")]
        pub interface_id: String,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct InterfaceVirtualPort {
        pub parameters: InterfaceVirtualPortParam,
    }

    #[derive(Debug, Deserialize)]
    pub struct Interface {
        pub source: InterfaceSource,
        #[serde(default)]
        pub target: InterfaceTarget,
        #[serde(default, rename = "virtualport")]
        pub virtual_port: InterfaceVirtualPort,
    }

    #[derive(Debug, Deserialize)]
    pub struct Devices {
        #[serde(rename = "disk")]
        pub disks: Vec<Disk>,
        #[serde(rename = "interface")]
        pub interfaces: Vec<Interface>,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct Flavor {
        pub name: String,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct User {
        pub name: String,
        pub uuid: String,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct Project {
        pub name: String,
        pub uuid: String,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct Owner {
        pub user: User,
        pub project: Project,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct Root {
        #[serde(rename = "type")]
        pub typ: String,
        pub uuid: String,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct Instance {
        pub flavor: Flavor,
        pub owner: Owner,
        pub name: String,
        pub root: Root,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct Metadata {
        pub instance: Instance,
    }

    #[derive(Debug, Deserialize)]
    pub struct Domain {
        pub devices: Devices,
        #[serde(default)]
        pub metadata: Metadata,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
