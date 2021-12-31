use crate::config::{
    default_std_interval, deserialize_std_duration, serialize_std_duration, ticker_from_duration,
    ticker_from_std_duration, DataType, GenerateConfig, SourceConfig, SourceContext,
    SourceDescription,
};
use crate::sources::Source;
use bitflags::bitflags;
use event::{tags, Metric};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::time::Duration;
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

#[derive(Debug, Deserialize, Serialize)]
struct LibvirtSourceConfig {
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
        let output = ctx.out.sink_map_err(|err| {
            warn!(message = "Error sending libvirt metrics", ?err,);
        });

        Ok(Box::pin(async move {
            while let Some(_ts) = ticker.next().await {}
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
    let conn = virt::connect::Connect::open(uri)?;

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
            | DomainStatsTypes::VIR_DOMAIN_STATS_CPU_TOTAL)
            .bits(),
        (DomainStatsFlags::VIR_CONNECT_LIST_DOMAINS_RUNNING
            | DomainStatsFlags::VIR_CONNECT_LIST_DOMAINS_SHUTOFF)
            .bits(),
    )?;

    for stat in &stats {}

    Ok(metrics)
}

mod schema {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize)]
    pub struct DiskTarget {
        pub dev: String,
        pub bus: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Disk {
        pub device: String,
        #[serde(rename = "type")]
        pub disk_type: String,
        pub target: DiskTarget,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct InterfaceSource {
        #[serde(default)]
        pub bridge: String,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct InterfaceTarget {
        pub device: String,
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

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Flavor {
        pub name: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct User {
        pub name: String,
        pub uuid: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Project {
        pub name: String,
        pub uuid: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Owner {
        pub user: User,
        pub project: Project,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Root {
        #[serde(rename = "type")]
        pub typ: String,
        pub uuid: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Instance {
        pub flavor: Flavor,
        pub owner: Owner,
        pub name: String,
        pub root: Root,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Metadata {
        pub instance: Instance,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Domain {
        pub devices: Devices,
        pub metadata: Metadata,
    }
}

fn domain_stat_to_metrics(stat: DomainStatsRecord) -> Result<Vec<Metric>, virt::error::Error> {
    let dom = unsafe { virt::domain::Domain::new((*stat.ptr).dom) };
    let name = dom.get_name()?;
    let uuid = dom.get_uuid_string()?;
    let info = dom.get_info()?;

    // Decode XML description of domain to get block device names, etc
    let xml_desc = dom.get_xml_desc(0)?;
    println!("{}:\n{}", name, xml_desc);

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
            )
        ),
        Metric::gauge_with_tags(
            "libvirt_domain_info_virtual_cpus",
            "Number of virtual CPUs for the domain",
            info.nr_virt_cpu,
            tags!(
                "domain" => &name,
            )
        ),
        Metric::sum_with_tags(
            "libvirt_domain_info_cpu_time_seconds_total",
            "Amount of CPU time used by the domain, in seconds",
            info.cpu_time /1000/1000/1000, // From ns to s
            tags!(
                "domain" => &name,
            )
        ),
        Metric::gauge_with_tags(
            "libvirt_domain_info_vstate",
            "Virtual domain state. 0: no, 1: running, 2: blocked, 3: paused, 4: shutdown, 5: shutoff, 6: crashed, 7: suspended",
            info.state as u32,
            tags!(
                "domain" => &name,
            )
        )
    ];

    // TODO: vcpu

    // TODO: block devices

    // TODO: network interface

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
                    | DomainStatsTypes::VIR_DOMAIN_STATS_CPU_TOTAL)
                    .bits(),
                (DomainStatsFlags::VIR_CONNECT_LIST_DOMAINS_RUNNING
                    | DomainStatsFlags::VIR_CONNECT_LIST_DOMAINS_SHUTOFF)
                    .bits(),
            )
            .unwrap();

        for stat in stats {
            // let domain = virt::domain::Domain::new(stat.ptr.dom);

            domain_stat_to_metrics(stat).unwrap();
        }
    }
}
