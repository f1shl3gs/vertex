use crate::config::{
    default_std_interval, deserialize_std_duration, serialize_std_duration,
    ticker_from_std_duration, DataType, GenerateConfig, SourceConfig, SourceContext,
    SourceDescription,
};
use crate::sources::Source;
use bitflags::bitflags;
use event::{tags, Event, Metric};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use virt::domain::DomainStatsRecord;
use virt::error::ErrorLevel;

// See also https://libvirt.org/html/libvirt-libvirt-domain.html#virDomainStatsTypes
bitflags! {
    struct DomainStatsTypes: u32 {
        const VIR_DOMAIN_STATS_STATE            = 0x1; // return domain state
        const VIR_DOMAIN_STATS_CPU_TOTAL        = 0x2; // return domain CPU info
        const VIR_DOMAIN_STATS_BALLOON          = 0x4; // return domain balloon info
        const VIR_DOMAIN_STATS_VCPU	            = 0x8; // return domain virtual CPU info
        const VIR_DOMAIN_STATS_INTERFACE        = 0x10; // return domain interfaces info
        const VIR_DOMAIN_STATS_BLOCK            = 0x20; // return domain block info
        const VIR_DOMAIN_STATS_PERF	            = 0x40; // return domain perf event info
        const VIR_DOMAIN_STATS_IOTHREAD	        = 0x80; // return iothread poll info
        const VIR_DOMAIN_STATS_MEMORY           = 0x100; // return domain memory info
        const VIR_DOMAIN_STATS_DIRTYRATE        = 0x200; // return domain dirty rate info
    }
}

// See also https://libvirt.org/html/libvirt-libvirt-domain.html#virConnectGetAllDomainStatsFlags
bitflags! {
    struct DomainStatsFlags: u32 {
        const VIR_CONNECT_LIST_DOMAINS_ACTIVE	        =   0x1;
        const VIR_CONNECT_LIST_DOMAINS_INACTIVE	        =   0x2;
        const VIR_CONNECT_LIST_DOMAINS_PERSISTENT       =   0x4;
        const VIR_CONNECT_LIST_DOMAINS_TRANSIENT	    =   0x8;
        const VIR_CONNECT_LIST_DOMAINS_RUNNING          =   0x10;
        const VIR_CONNECT_LIST_DOMAINS_PAUSED           =   0x20;
        const VIR_CONNECT_LIST_DOMAINS_SHUTOFF          =   0x40;
        const VIR_CONNECT_LIST_DOMAINS_OTHER            =   0x80;
        const VIR_CONNECT_LIST_DOMAINS_MANAGEDSAVE	    =   0x100;
        const VIR_CONNECT_LIST_DOMAINS_NO_MANAGEDSAVE	=   0x200;
        const VIR_CONNECT_LIST_DOMAINS_AUTOSTART        =   0x400;
        const VIR_CONNECT_LIST_DOMAINS_NO_AUTOSTART     =   0x800;
        const VIR_CONNECT_LIST_DOMAINS_HAS_SNAPSHOT     =   0x1000;
        const VIR_CONNECT_LIST_DOMAINS_NO_SNAPSHOT      =   0x2000;
        const VIR_CONNECT_LIST_DOMAINS_HAS_CHECKPOINT   =   0x4000;
        const VIR_CONNECT_LIST_DOMAINS_NO_CHECKPOINT    =   0x8000;
    }
}

fn default_uri() -> String {
    "qemu:///system".to_string()
}

#[derive(Debug, Deserialize, Serialize)]
struct LibvirtSourceConfig {
    #[serde(default = "default_uri")]
    uri: String,
    #[serde(default = "default_std_interval")]
    #[serde(
        deserialize_with = "deserialize_std_duration",
        serialize_with = "serialize_std_duration"
    )]
    interval: Duration,
}

impl GenerateConfig for LibvirtSourceConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::to_value(LibvirtSourceConfig {
            uri: "".to_string(),
            interval: default_std_interval(),
        })
        .unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<LibvirtSourceConfig>("libvirt")
}

#[async_trait::async_trait]
#[typetag::serde(name = "libvirt")]
impl SourceConfig for LibvirtSourceConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let mut ticker = ticker_from_std_duration(self.interval).take_until(ctx.shutdown);
        let uri = self.uri.clone();
        let mut output = ctx.out.sink_map_err(|err| {
            warn!(message = "Error sending libvirt metrics", ?err);
        });

        Ok(Box::pin(async move {
            while let Some(_ts) = ticker.next().await {
                let turi = uri.clone();
                match tokio::task::spawn_blocking(move || {
                    let start = Instant::now();
                    let result = gather(&turi);
                    let up = result.is_ok();

                    let mut metrics = result.unwrap_or_default();
                    metrics.extend_from_slice(&[
                        Metric::gauge(
                            "libvirt_up",
                            "Whether scraping libvirt's metrics was successful",
                            up,
                        ),
                        Metric::gauge(
                            "libvirt_scrape_duration_seconds",
                            "",
                            start.elapsed().as_secs_f64(),
                        ),
                    ]);
                    metrics
                })
                .await
                {
                    Ok(mut metrics) => {
                        let timestamp = Some(chrono::Utc::now());
                        metrics.iter_mut().for_each(|m| m.timestamp = timestamp);
                        let _ = output
                            .send_all(
                                &mut futures::stream::iter(metrics).map(Event::Metric).map(Ok),
                            )
                            .await;
                    }
                    Err(err) => {
                        warn!(message = "Scrape libvirt metrics failed", ?err, uri = %uri);
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
        "libvirt"
    }
}

#[inline]
fn version_num_to_string(version_num: u32) -> String {
    format!(
        "{}.{}.{}",
        version_num / 1000000 % 1000,
        version_num / 1000 % 1000,
        version_num % 1000
    )
}

fn gather(uri: &str) -> Result<Vec<Metric>, virt::error::Error> {
    let conn = virt::connect::Connect::open_read_only(uri)?;

    // virConnectGetVersion, hypervisor running, e.g. QEMU
    let version = conn.get_hyp_version()?;
    let hyper_version = version_num_to_string(version);

    // virConnectGetLibVersion, libvirt daemon running
    let version = conn.get_lib_version()?;
    let libvirtd_version = version_num_to_string(version);

    // virGetVersion, version of libvirt(dynamic) library used by this binary,
    // not the daemon version.
    let version = virt::connect::Connect::get_version()?;
    let library_version = version_num_to_string(version);

    let mut metrics = vec![Metric::gauge_with_tags(
        "libvirt_version_info",
        "Versions of virtualization components",
        1,
        tags!(
            "hypervisor_running" => hyper_version,
            "libvirtd_running" => libvirtd_version,
            "libvirt_library" => library_version
        ),
    )];

    let stats = conn.get_all_domain_stats(
        (DomainStatsTypes::VIR_DOMAIN_STATS_STATE
            | DomainStatsTypes::VIR_DOMAIN_STATS_CPU_TOTAL
            | DomainStatsTypes::VIR_DOMAIN_STATS_INTERFACE
            | DomainStatsTypes::VIR_DOMAIN_STATS_BALLOON
            | DomainStatsTypes::VIR_DOMAIN_STATS_BLOCK
            | DomainStatsTypes::VIR_DOMAIN_STATS_PERF
            | DomainStatsTypes::VIR_DOMAIN_STATS_VCPU)
            .bits(),
        (DomainStatsFlags::VIR_CONNECT_LIST_DOMAINS_RUNNING
            | DomainStatsFlags::VIR_CONNECT_LIST_DOMAINS_SHUTOFF)
            .bits(),
    )?;

    for stat in &stats {
        let partial = domain_stat_to_metrics(stat)?;
        metrics.extend(partial);
    }

    // Collect pool info
    let pools = conn.list_all_storage_pools(2)?; // 2 for "ACTIVE" pools
    for pool in &pools {
        pool.refresh(0)?;

        let name = pool.get_name()?;
        let info = pool.get_info()?;
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
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Default, Deserialize, Serialize)]
    pub struct DiskTarget {
        pub dev: String,
        pub bus: String,
    }

    #[derive(Clone, Debug, Default, Deserialize, Serialize)]
    pub struct DiskSource {
        pub file: String,
        #[serde(default)]
        pub name: String,
    }

    #[derive(Clone, Debug, Default, Deserialize, Serialize)]
    pub struct DiskDriver {
        #[serde(rename = "type")]
        pub typ: String,
        #[serde(default)]
        pub cache: String,
        #[serde(default)]
        pub discard: String,
    }

    #[derive(Clone, Debug, Default, Deserialize, Serialize)]
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

    #[derive(Debug, Deserialize, Serialize)]
    pub struct InterfaceSource {
        #[serde(default)]
        pub bridge: String,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct InterfaceTarget {
        pub dev: String,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct InterfaceVirtualPortParam {
        #[serde(rename = "interfaceid")]
        pub interface_id: String,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct InterfaceVirtualPort {
        pub parameters: InterfaceVirtualPortParam,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Interface {
        pub source: InterfaceSource,
        #[serde(default)]
        pub target: InterfaceTarget,
        #[serde(default, rename = "virtualport")]
        pub virtual_port: InterfaceVirtualPort,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Devices {
        #[serde(rename = "disk")]
        pub disks: Vec<Disk>,
        #[serde(rename = "interface")]
        pub interfaces: Vec<Interface>,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Flavor {
        pub name: String,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct User {
        pub name: String,
        pub uuid: String,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Project {
        pub name: String,
        pub uuid: String,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Owner {
        pub user: User,
        pub project: Project,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Root {
        #[serde(rename = "type")]
        pub typ: String,
        pub uuid: String,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Instance {
        pub flavor: Flavor,
        pub owner: Owner,
        pub name: String,
        pub root: Root,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Metadata {
        pub instance: Instance,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Domain {
        pub devices: Devices,
        #[serde(default)]
        pub metadata: Metadata,
    }

    #[derive(Debug, Default)]
    pub struct MemoryStats {
        pub major_fault: u64,
        pub minor_fault: u64,
        pub unused: u64,
        pub available: u64,
        pub actual_balloon: u64,
        pub rss: u64,
        pub usable: u64,
        pub disk_caches: u64,
    }

    impl From<Vec<virt::domain::MemoryStats>> for MemoryStats {
        fn from(array: Vec<virt::domain::MemoryStats>) -> Self {
            let mut stats = Self::default();

            for s in &array {
                match s.tag {
                    2 => stats.major_fault = s.val,
                    3 => stats.minor_fault = s.val,
                    4 => stats.unused = s.val,
                    5 => stats.available = s.val,
                    6 => stats.actual_balloon = s.val,
                    7 => stats.rss = s.val,
                    8 => stats.usable = s.val,
                    10 => stats.disk_caches = s.val,
                    _ => { /* do nothing */ }
                }
            }

            stats
        }
    }
}

fn domain_stat_to_metrics(stat: &DomainStatsRecord) -> Result<Vec<Metric>, virt::error::Error> {
    let dom = unsafe { virt::domain::Domain::new((*stat.ptr).dom) };
    let name = dom.get_name()?;
    let uuid = dom.get_uuid_string()?;
    let info = dom.get_info()?;

    // Decode XML description of domain to get block device names, etc
    let xml_desc = dom.get_xml_desc(0)?;
    let schema::Domain { devices, metadata } = serde_xml_rs::from_str::<schema::Domain>(&xml_desc)
        .map_err(|err| virt::error::Error {
            code: 0,
            domain: 0,
            message: format!("{:?}", err),
            level: ErrorLevel::NONE,
        })?;

    // Report domain info
    let mut metrics = vec![
        Metric::gauge_with_tags(
            "libvirt_domain_info_meta",
            "Domain metadata",
            1,
            tags!(
                "domain" => &name,
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
                "domain" => &name
            ),
        ),
        Metric::gauge_with_tags(
            "libvirt_domain_info_memory_usage_bytes",
            "Memory usage of the domain, in bytes",
            info.memory * 1024,
            tags!(
                "domain" => &name
            ),
        ),
        Metric::gauge_with_tags(
            "libvirt_domain_info_virtual_cpus",
            "Number of virtual CPUs for the domain",
            info.nr_virt_cpu,
            tags!(
                "domain" => &name,
            ),
        ),
        Metric::sum_with_tags(
            "libvirt_domain_info_cpu_time_seconds_total",
            "Amount of CPU time used by the domain, in seconds",
            info.cpu_time as f64 / 1000.0 / 1000.0 / 1000.0, // From ns to s
            tags!(
                "domain" => &name,
            ),
        ),
        Metric::gauge_with_tags(
            "libvirt_domain_info_vstate",
            "Virtual domain state. 0: no, 1: running, 2: blocked, 3: paused, 4: shutdown, 5: shutoff, 6: crashed, 7: suspended",
            info.state as u32,
            tags!(
                "domain" => &name,
            ),
        ),
    ];

    // Report vcpus
    let vcpus = dom.get_vcpus()?;
    for vcpu in vcpus {
        let vcpu_num = vcpu.number.to_string();

        // There is no Wait in GetVcpus(), But there's no cpu number in
        // DomainStats Time and State are present in both structs.
        // So, let's take Wait here
        let (wait, delay) = stat.vcpu_wait_and_delay(vcpu.number)?;

        metrics.extend_from_slice(&[
            Metric::gauge_with_tags(
                "libvirt_domain_vcpu_state",
                "VCPU state. 0: offline, 1: running, 2: blocked",
                vcpu.state,
                tags!(
                    "domain" => &name,
                    "vcpu" => &vcpu_num,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_vcpu_time_seconds_total",
                "Amount of CPU time used by the domain's VCPU, in seconds",
                vcpu.cpu_time as f64 / 1000.0 / 1000.0 / 1000.0, // From ns to s
                tags!(
                    "domain" => &name,
                    "vcpu" => &vcpu_num
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_vcpu_cpu",
                "Real CPU number, or one of the values from virVcpuHostCpuState",
                vcpu.cpu,
                tags!(
                    "domain" => &name,
                    "vcpu" => &vcpu_num
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_vcpu_wait_seconds_total",
                "Vcpu's wait_sum metrics. CONFIG_SCHEDSTATS has to be enabled",
                wait as f64 / 1000.0 / 1000.0 / 1000.0,
                tags!(
                    "domain" => &name,
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
                    "domain" => &name,
                    "vcpu" => &vcpu_num
                ),
            ),
        ]);
    }

    // Report block device statistics
    let blocks = stat.block_stats()?;
    for block in &blocks {
        // Ugly hack to avoid getting metrics from cdrom block device
        // TODO: somehow check the disk 'device' field for 'cdrom' string
        if block.name == "hdc" || block.name == "hda" {
            continue;
        }

        let dev = devices
            .disks
            .iter()
            .find(|dev| dev.target.dev == block.name)
            .map(|d| d.clone())
            .unwrap_or_default();

        let disk_source = if block.path != "" {
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
                    "domain" => &name,
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
                    "domain" => &name,
                    "target_device" => &block.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_block_stats_read_requests_total",
                "Number of read requests from a block device",
                block.read_requests,
                tags!(
                    "domain" => &name,
                    "target_device" => &block.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_block_stats_read_time_seconds_total",
                "Total time spent on reads from a block device, in seconds",
                block.read_time as f64 / 1000.0 / 1000.0 / 1000.0, // From ns to s
                tags!(
                    "domain" => &name,
                    "target_device" => &block.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_block_stats_write_bytes_total",
                "Number of bytes written to a block device, in bytes",
                block.write_bytes,
                tags!(
                    "domain" => &name,
                    "target_device" => &block.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_block_stats_write_requests_total",
                "Number of write requests to a block device",
                block.write_requests,
                tags!(
                    "domain" => &name,
                    "target_device" => &block.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_block_stats_write_time_seconds_total",
                "Total time spent on writes on a block device, in seconds",
                block.write_time as f64 / 1000.0 / 1000.0 / 1000.0, // From ns to s
                tags!(
                    "domain" => &name,
                    "target_device" => &block.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_block_stats_flush_requests_total",
                "Total flush requests from a block device",
                block.flush_requests,
                tags!(
                    "domain" => &name,
                    "target_device" => &block.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_block_stats_flush_time_seconds_total",
                "Total time in seconds spent on cache flushing to a block device",
                block.flush_time as f64 / 1000.0 / 1000.0 / 1000.0, // From ns to s
                tags!(
                    "domain" => &name,
                    "target_device" => &block.name,
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_block_stats_allocation",
                "Offset of the highest written sector on a block device",
                block.allocation,
                tags!(
                    "domain" => &name,
                    "target_device" => &block.name,
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_block_stats_capacity_bytes",
                "Logical size in bytes of the block device backing image",
                block.capacity,
                tags!(
                    "domain" => &name,
                    "target_device" => &block.name,
                ),
            ),
            Metric::gauge_with_tags(
                "libvirt_domain_block_stats_physical_bytes",
                "Physical size in bytes of the container of the backing image",
                block.physical,
                tags!(
                    "domain" => &name,
                    "target_device" => &block.name,
                ),
            ),
        ]);

        match stat.block_io_tune(&block.name) {
            Ok(params) => metrics.extend_from_slice(&[
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_total_bytes",
                    "Total throughput limit in bytes per second",
                    params.total_bytes_sec,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_read_bytes",
                    "Read throughput limit in bytes per second",
                    params.read_bytes_sec,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_write_bytes",
                    "Write throughput limit in bytes per second",
                    params.write_bytes_sec,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_total_requests",
                    "Total requests per second limit",
                    params.total_iops_sec,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_read_requests",
                    "Read requests per second limit",
                    params.read_iops_sec,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_write_requests",
                    "Write requests per second limit",
                    params.write_iops_sec,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_burst_total_bytes",
                    "Total throughput burst limit in bytes per second",
                    params.total_bytes_sec_max,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_burst_read_bytes",
                    "Read throughput burst limit in bytes per second",
                    params.read_bytes_sec_max,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_burst_write_bytes",
                    "Write throughput burst limit in bytes per second",
                    params.write_bytes_sec_max,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_burst_total_requests",
                    "Total requests per second burst limit",
                    params.total_iops_sec_max,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_burst_total_bytes_length_seconds",
                    "Total throughput burst time in seconds",
                    params.total_bytes_sec_max_length,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_burst_read_bytes_length_seconds",
                    "Read throughput burst time in seconds",
                    params.read_bytes_sec_max_length,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_burst_write_bytes_length_seconds",
                    "Write throughput burst time in seconds",
                    params.write_bytes_sec_max_length,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_burst_length_total_requests_seconds",
                    "Total requests per second burst time in seconds",
                    params.total_iops_sec_max_length,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_burst_length_read_requests_seconds",
                    "Read requests per second burst time in seconds",
                    params.read_iops_sec_max_length,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_limit_burst_length_write_requests_seconds",
                    "Write requests per second burst time in seconds",
                    params.write_bytes_sec_max_length,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
                Metric::gauge_with_tags(
                    "libvirt_domain_block_stats_size_iops_bytes",
                    "The size of IO operations per seconds permitted through a block device",
                    params.size_iops_sec,
                    tags!(
                        "domain" => &name,
                        "target_device" => &block.name,
                    ),
                ),
            ]),
            Err(err) => match err.code {
                // See also: https://github.com/libvirt/libvirt/blob/56fbabf1a1e272c6cc50adcb603996cf8e94ad08/include/libvirt/virterror.h#L209
                55 => {
                    warn!(message = "Invalid operation get_block_io_tune", ?err);
                }
                84 => {
                    warn!(message = "Unsupported operation get_block_io_tune", ?err);
                }
                _ => return Err(err),
            },
        }
    }

    // Report network interface statistics
    for iface in stat.network_stats()? {
        let mut source_bridge = "";
        let mut virtual_interface = "";
        for net in &devices.interfaces {
            if net.target.dev == iface.name {
                source_bridge = &net.source.bridge;
                virtual_interface = &net.virtual_port.parameters.interface_id;
            }
        }

        if source_bridge != "" || virtual_interface != "" {
            metrics.push(Metric::gauge_with_tags(
                "libvirt_domain_interface_meta",
                "Interfaces metadata. Source bridge, target device, interface uuid",
                1,
                tags!(
                    "domain" => &name,
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
                    "domain" => &name,
                    "target_device" => &iface.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_interface_stats_receive_packets_total",
                "Number of packets received on a network interface",
                iface.rx_packets,
                tags!(
                    "domain" => &name,
                    "target_device" => &iface.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_interface_stats_receive_errors_total",
                "Number of packet receive errors on a network interface",
                iface.rx_errs,
                tags!(
                    "domain" => &name,
                    "target_device" => &iface.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_interface_stats_receive_drop_total",
                "Number of packets receive drops on a network interface",
                iface.rx_bytes,
                tags!(
                    "domain" => &name,
                    "target_device" => &iface.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_interface_stats_transmit_bytes_total",
                "Number of bytes transmitted on a network interface, in bytes",
                iface.tx_bytes,
                tags!(
                    "domain" => &name,
                    "target_device" => &iface.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_interface_stats_transmit_packets_total",
                "Number of packets transmitted on a network interface",
                iface.tx_packets,
                tags!(
                    "domain" => &name,
                    "target_device" => &iface.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_interface_stats_transmit_errors_total",
                "Number of packet transmit errors on a network interface",
                iface.tx_errs,
                tags!(
                    "domain" => &name,
                    "target_device" => &iface.name,
                ),
            ),
            Metric::sum_with_tags(
                "libvirt_domain_interface_stats_transmit_drop_total",
                "Number of packets transmit drops on a network interface",
                iface.tx_bytes,
                tags!(
                    "domain" => &name,
                    "target_device" => &iface.name,
                ),
            ),
        ])
    }

    // Collect Memory Stats
    let ms = dom.memory_stats(11, 0)?;
    let stats: schema::MemoryStats = ms.into();
    let used_percent = if stats.usable != 0 && stats.available != 0 {
        ((stats.available - stats.usable) / stats.available / 100) as f64
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
                "domain" => &name,
            ),
        ),
        Metric::sum_with_tags(
            "libvirt_domain_memory_stats_minor_fault_total",
            "Page faults occur when a process makes a valid access to virtual memory that is not available.\
             When servicing the page not fault, if disk IO is required, it is considered a minor fault.",
            stats.minor_fault,
            tags!(
                "domain" => &name,
            ),
        ),
        Metric::gauge_with_tags(
            "libvirt_domain_memory_stats_unused_bytes",
            "The amount of memory left completely unused by the system. Memory that is \
            available but used for reclaimable cache should NOT be reported as free. This value \
            is expressed in bytes",
            stats.unused * 1024,
            tags!(
                "domain" => &name,
            ),
        ),
        Metric::gauge_with_tags(
            "libvirt_domain_memory_stats_available_bytes",
            "The total amount of usable memory as seen by the domain. This value may be less \
            than the amount of memory assigned to the domain if a balloon driver is in use or if \
            the guest OS does not initialize all assigned pages. This value is expressed in bytes",
            stats.available * 1024,
            tags!(
                "domain" => &name,
            ),
        ),
        Metric::gauge_with_tags(
            "libvirt_domain_memory_stats_actual_balloon_bytes",
            "Current balloon value (in bytes)",
            stats.actual_balloon * 1024,
            tags!(
                "domain" => &name,
            ),
        ),
        Metric::gauge_with_tags(
            "libvirt_domain_memory_stats_rss_bytes",
            "Resident Set Size of the process running the domain. This value is in bytes",
            stats.rss * 1024,
            tags!(
                "domain" => &name,
            ),
        ),
        Metric::gauge_with_tags(
            "libvirt_domain_memory_stats_usable_bytes",
            "How much the balloon can be inflated without pushing the guest system to swap, \
            corresponds to 'Available' in /proc/meminfo",
            stats.usable * 1024,
            tags!(
                "domain" => &name,
            ),
        ),
        Metric::gauge_with_tags(
            "libvirt_domain_memory_stats_disk_cache_bytes",
            "The amount of memory, that can be quickly reclaimed without additional I/O \
            (in bytes). Typically these pages are used for caching files from disk",
            stats.disk_caches * 1024,
            tags!(
                "domain" => &name,
            ),
        ),
        Metric::gauge_with_tags(
            "libvirt_domain_memory_stats_used_percent",
            "The amount of memory in percent, that used by domain",
            used_percent,
            tags!(
                "domain" => &name,
            ),
        )
    ]);

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::print_stdout)] // tests
    use super::*;

    #[test]
    #[ignore]
    fn get_all_domain_stats() {
        let uri = "qemu:///system";
        let conn = virt::connect::Connect::open(uri).unwrap();

        let stats = conn
            .get_all_domain_stats(
                (DomainStatsTypes::VIR_DOMAIN_STATS_STATE
                    | DomainStatsTypes::VIR_DOMAIN_STATS_CPU_TOTAL
                    | DomainStatsTypes::VIR_DOMAIN_STATS_INTERFACE
                    | DomainStatsTypes::VIR_DOMAIN_STATS_BALLOON
                    | DomainStatsTypes::VIR_DOMAIN_STATS_BLOCK
                    | DomainStatsTypes::VIR_DOMAIN_STATS_PERF
                    | DomainStatsTypes::VIR_DOMAIN_STATS_VCPU)
                    .bits(),
                (DomainStatsFlags::VIR_CONNECT_LIST_DOMAINS_RUNNING
                    | DomainStatsFlags::VIR_CONNECT_LIST_DOMAINS_SHUTOFF)
                    .bits(),
            )
            .unwrap();

        for stat in &stats {
            let metrics = domain_stat_to_metrics(stat).unwrap();
            for metric in metrics {
                println!("{}", metric);
            }
        }
    }
}
