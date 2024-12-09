#![allow(dead_code)]

use std::io::{Cursor, Read};
use std::net::{IpAddr, Ipv4Addr};

use bytes::Buf;

/// A simple helper for decode
pub trait ReadExt: Read {
    fn read_u16(&mut self) -> std::io::Result<u16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    fn read_u32(&mut self) -> std::io::Result<u32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    fn read_u64(&mut self) -> std::io::Result<u64> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }
}

impl ReadExt for Cursor<&[u8]> {}

// Opaque sample_data types according to https://sflow.org/SFLOW-DATAGRAM5.txt
const SAMPLE_FORMAT_FLOW: u32 = 1;
const SAMPLE_FORMAT_COUNTER: u32 = 2;
const SAMPLE_FORMAT_EXPANDED_FLOW: u32 = 3;
const SAMPLE_FORMAT_EXPANDED_COUNTER: u32 = 4;
const SAMPLE_FORMAT_DROP: u32 = 5;

// Opaque flow_data types according to https://sflow.org/SFLOW-STRUCTS5.txt
const FLOW_TYPE_RAW: u32 = 1;
const FLOW_TYPE_ETH: u32 = 2;
const FLOW_TYPE_IPV4: u32 = 3;
const FLOW_TYPE_IPV6: u32 = 4;
const FLOW_TYPE_EXT_SWITCH: u32 = 1001;
const FLOW_TYPE_EXT_ROUTER: u32 = 1002;
const FLOW_TYPE_EXT_GATEWAY: u32 = 1003;
const FLOW_TYPE_EXT_USER: u32 = 1004;
const FLOW_TYPE_EXT_URL: u32 = 1005;
const FLOW_TYPE_EXT_MPLS: u32 = 1006;
const FLOW_TYPE_EXT_NAT: u32 = 1007;
const FLOW_TYPE_EXT_MPLS_TUNNEL: u32 = 1008;
const FLOW_TYPE_EXT_MPLS_VC: u32 = 1009;
const FLOW_TYPE_EXT_MPLS_FEC: u32 = 1010;
const FLOW_TYPE_EXT_MPLS_LVP_FEC: u32 = 1011;
const FLOW_TYPE_EXT_VLAN_TUNNEL: u32 = 1012;

// According to https://sflow.org/sflow_drops.txt
const FLOW_TYPE_EGRESS_QUEUE: u32 = 1036;
const FLOW_TYPE_EXT_ACL: u32 = 1037;
const FLOW_TYPE_EXT_FUNCTION: u32 = 1038;

// Opaque counter_data types according to https://sflow.org/SFLOW-STRUCTS5.txt
const COUNTER_TYPE_IF: u32 = 1;
const COUNTER_TYPE_ETH: u32 = 2;
const COUNTER_TYPE_TOKENRING: u32 = 3;
const COUNTER_TYPE_VG: u32 = 4;
const COUNTER_TYPE_VLAN: u32 = 5;
const COUNTER_TYPE_CPU: u32 = 1001;

const COUNTER_TYPE_HOST_DESCRIPTION: u32 = 2000;
const COUNTER_TYPE_HOST_ADAPTERS: u32 = 2001;
const COUNTER_TYPE_HOST_CPU: u32 = 2003;
const COUNTER_TYPE_HOST_MEMORY: u32 = 2004;
const COUNTER_TYPE_HOST_DISK_IO: u32 = 2005;
const COUNTER_TYPE_HOST_NET_IO: u32 = 2006;
const COUNTER_TYPE_MIB2_IP_GROUP: u32 = 2007;
const COUNTER_TYPE_MIB2_ICMP_GROUP: u32 = 2008;
const COUNTER_TYPE_MIB2_TCP_GROUP: u32 = 2009;
const COUNTER_TYPE_MIB2_UDP_GROUP: u32 = 2010;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("incompatible version")]
    IncompatibleVersion,
    #[error("unknown ip version found {0}")]
    UnknownIpVersion(u32),
    #[error("too many samples")]
    TooManySamples,
    #[error(transparent)]
    Io(std::io::Error),
    #[error("unknown sample format found {0}")]
    UnknownSampleFormat(u32),
    #[error("too many flow records")]
    TooManyFlowRecords,
    #[error("too many AS path")]
    TooManyAsPath,
    #[error("invalid AS path length")]
    InvalidAsPathLength,
    #[error("too many communities")]
    TooManyCommunities,
    #[error("invalid communities length")]
    InvalidCommunitiesLength,
    #[error("unsupported flow record type found {0}")]
    UnsupportedFlowRecordType(u32),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

#[derive(Debug)]
pub struct SampleHeader {
    format: u32,
    length: u32,

    sample_sequence_number: u32,
    source_id_type: u32,
    source_id_value: u32,
}

#[derive(Debug)]
pub struct RecordHeader {
    data_format: u32,
    length: u32,
}

#[derive(Debug)]
pub struct FlowRecordRaw {
    pub protocol: u32,
    pub frame_length: u32,
    pub stripped: u32,
    pub original_length: u32,
    pub header_data: Vec<u8>,
}

#[derive(Debug)]
pub struct FlowRecordSampleEthernet {
    pub length: u32,
    pub src_mac: [u8; 6],
    pub dst_mac: [u8; 6],
    pub eth_type: u32,
}

#[derive(Debug)]
pub struct SampledIpv4 {
    pub length: u32,
    pub protocol: u32,
    pub src_ip: Ipv4Addr,
    pub dst_ip: Ipv4Addr,
    pub src_port: u32,
    pub dst_port: u32,
    pub tcp_flags: u32,

    pub tos: u32,
}

#[derive(Debug)]
pub struct SampledIpv6 {
    pub length: u32,
    pub protocol: u32,
    pub src_ip: Ipv4Addr,
    pub dst_ip: Ipv4Addr,
    pub src_port: u32,
    pub dst_port: u32,
    pub tcp_flags: u32,

    pub priority: u32,
}

#[derive(Debug)]
pub struct ExtendedSwitch {
    pub src_vlan: u32,
    pub src_priority: u32,
    pub dst_vlan: u32,
    pub dst_priority: u32,
}

#[derive(Debug)]
pub struct ExtendedRouter {
    pub next_hop_ip_version: u32,
    pub next_hop: IpAddr,
    pub src_mask_len: u32,
    pub dst_mask_len: u32,
}

#[derive(Debug)]
pub struct ExtendedGateway {
    pub next_hop_ip_version: u32,
    pub next_hop: IpAddr,
    pub r#as: u32,
    pub src_as: u32,
    pub src_peer_as: u32,
    pub as_destinations: u32,
    pub as_path_type: u32,
    pub as_path_length: u32,
    pub as_path: Vec<u32>,
    pub communities_length: u32,
    pub communities: Vec<u32>,
    pub local_pref: u32,
}

#[derive(Debug)]
pub struct EgressQueue {
    pub queue: u32,
}

#[derive(Debug)]
pub struct ExtendedACL {
    pub number: u32,
    pub name: String,
    pub direction: u32,
}

#[derive(Debug)]
pub struct ExtendedFunction {
    pub symbol: String,
}

#[derive(Debug)]
pub enum FlowRecord {
    Raw(FlowRecordRaw),
    SampledEthernet(FlowRecordSampleEthernet),
    SampledIpv4(SampledIpv4),
    SampledIpv6(SampledIpv6),
    ExtendedSwitch(ExtendedSwitch),
    ExtendedRouter(ExtendedRouter),
    ExtendedGateway(ExtendedGateway),
    EgressQueue(EgressQueue),
    ExtendedACL(ExtendedACL),
    ExtendedFunction(ExtendedFunction),
}

#[derive(Debug)]
pub struct IfCounters {
    pub if_index: u32,
    pub if_type: u32,
    pub if_speed: u64,
    pub if_direction: u32,
    pub if_status: u32,
    pub if_in_octets: u64,
    pub if_in_ucast_pkts: u32,
    pub if_in_multicast_pkts: u32,
    pub if_in_broadcast_pkts: u32,
    pub if_in_discards: u32,
    pub if_in_errors: u32,
    pub if_in_unknown_protos: u32,
    pub if_out_octets: u64,
    pub if_out_ucast_pkts: u32,
    pub if_out_multicast_pkts: u32,
    pub if_out_broadcast_pkts: u32,
    pub if_out_discards: u32,
    pub if_out_errors: u32,
    pub if_promiscuous_mode: u32,
}

#[derive(Debug)]
pub struct EthernetCounters {
    pub dot3stats_alignment_errors: u32,
    pub dot3stats_fcserrors: u32,
    pub dot3stats_single_collision_frames: u32,
    pub dot3stats_multiple_collision_frames: u32,
    pub dot3stats_sqetest_errors: u32,
    pub dot3stats_deferred_transmissions: u32,
    pub dot3stats_late_collisions: u32,
    pub dot3stats_excessive_collisions: u32,
    pub dot3stats_internal_mac_transmit_errors: u32,
    pub dot3stats_carrier_sense_errors: u32,
    pub dot3stats_frame_too_longs: u32,
    pub dot3stats_internal_mac_receive_errors: u32,
    pub dot3stats_symbol_errors: u32,
}

#[derive(Debug)]
pub struct HostAdapter {
    pub if_index: u32,
    pub mac_addresses: Vec<[u8; 6]>,
}

#[derive(Debug)]
pub struct HostAdapters {
    pub length: u32,
    pub adapters: Vec<HostAdapter>,
}

#[derive(Debug)]
pub struct HostDescription {
    pub host: String,
    pub uuid: [u8; 16],
    pub machine_type: u32,
    pub os_name: u32,
    pub os_release: String,
}

#[derive(Debug)]
pub struct HostCPU {
    pub load_one: f32,     /* 1 minute load avg., -1.0 = unknown */
    pub load_five: f32,    /* 5 minute load avg., -1.0 = unknown */
    pub load_fifteen: f32, /* 15 minute load avg., -1.0 = unknown */

    pub proc_run: u32,   /* total number of running processes */
    pub proc_total: u32, /* total number of processes */
    pub cpu_num: u32,    /* number of CPUs */
    pub cpu_speed: u32,  /* speed in MHz of CPU */
    pub uptime: u32,     /* seconds since last reboot */
    pub cpu_user: u32,   /* user time (ms) */
    pub cpu_nice: u32,   /* nice time (ms) */
    pub cpu_system: u32, /* system time (ms) */
    pub cpu_idle: u32,   /* idle time (ms) */
    pub cpu_wio: u32,    /* time waiting for I/O to complete (ms) */
    pub cpu_intr: u32,   /* time servicing interrupts (ms) */
    pub cpu_sintr: u32,  /* time servicing soft interrupts (ms) */
    pub interrupts: u32, /* interrupt count */
    pub contexts: u32,   /* context switch count */

    // theos fields might not empty
    pub cpu_steal: u32,
    pub cpu_guest: u32,
    pub cpu_guest_nice: u32,
}

#[derive(Debug)]
pub struct HostMemory {
    pub mem_total: u64,   /* total kB */
    pub mem_free: u64,    /* free kB */
    pub mem_shared: u64,  /* shared kB */
    pub mem_buffers: u64, /* buffers kB */
    pub mem_cached: u64,  /* cached kB */
    pub swap_total: u64,  /* swap total kB */
    pub swap_free: u64,   /* swap free kB */
    pub page_in: u32,     /* page in count */
    pub page_out: u32,    /* page out count */
    pub swap_in: u32,     /* swap in count */
    pub swap_out: u32,    /* swap out count */
}

#[derive(Debug)]
#[repr(C, packed)]
pub struct HostDiskIO {
    pub disk_total: u64,    /* total disk size in bytes */
    pub disk_free: u64,     /* total disk free in bytes */
    pub part_max_used: u32, /* utilization of most utilized partition */

    pub reads: u32,      /* reads issued */
    pub bytes_read: u64, /* bytes read */
    pub read_time: u32,  /* read time (ms) */

    pub writes: u32,        /* writes completed */
    pub bytes_written: u64, /* bytes written */
    pub write_time: u32,    /* write time (ms) */
}

#[derive(Debug)]
pub struct HostNetIO {
    pub bytes_in: u64,    /* total bytes in */
    pub pkts_in: u32,     /* total packets in */
    pub errs_in: u32,     /* total errors in */
    pub drops_in: u32,    /* total drops in */
    pub bytes_out: u64,   /* total bytes out */
    pub packets_out: u32, /* total packets out */
    pub errs_out: u32,    /* total errors out */
    pub drops_out: u32,   /* total drops out */
}

#[derive(Debug)]
pub struct Mib2IpGroup {
    pub ip_forwarding: u32,
    pub ip_default_ttl: u32,
    pub ip_in_receives: u32,
    pub ip_in_hdr_errors: u32,
    pub ip_in_addr_errors: u32,
    pub ip_forw_datagrams: u32,
    pub ip_in_unknown_protos: u32,
    pub ip_in_discards: u32,
    pub ip_in_delivers: u32,
    pub ip_out_requests: u32,
    pub ip_out_discards: u32,
    pub ip_out_no_routes: u32,
    pub ip_reasm_timeout: u32,
    pub ip_reasm_reqds: u32,
    pub ip_reasm_oks: u32,
    pub ip_reasm_fails: u32,
    pub ip_frag_oks: u32,
    pub ip_frag_fails: u32,
    pub ip_frag_creates: u32,
}

#[derive(Debug)]
pub struct Mib2IcmpGroup {
    pub icmp_in_msgs: u32,
    pub icmp_in_errors: u32,
    pub icmp_in_dest_unreachs: u32,
    pub icmp_in_time_excds: u32,
    pub icmp_in_param_probs: u32,
    pub icmp_in_src_quenchs: u32,
    pub icmp_in_redirects: u32,
    pub icmp_in_echos: u32,
    pub icmp_in_echo_reps: u32,
    pub icmp_in_timestamps: u32,
    pub icmp_in_addr_masks: u32,
    pub icmp_in_addr_mask_reps: u32,
    pub icmp_out_msgs: u32,
    pub icmp_out_errors: u32,
    pub icmp_out_dest_unreachs: u32,
    pub icmp_out_time_excds: u32,
    pub icmp_out_param_probs: u32,
    pub icmp_out_src_quenchs: u32,
    pub icmp_out_redirects: u32,
    pub icmp_out_echos: u32,
    pub icmp_out_echo_reps: u32,
    pub icmp_out_timestamps: u32,
    pub icmp_out_timestamp_reps: u32,
    pub icmp_out_addr_masks: u32,
    pub icmp_out_addr_mask_reps: u32,
}

#[derive(Debug)]
pub struct Mib2TcpGroup {
    pub tcp_rto_algorithm: u32,
    pub tcp_rto_min: u32,
    pub tcp_rto_max: u32,
    pub tcp_max_conn: u32,
    pub tcp_active_opens: u32,
    pub tcp_passive_opens: u32,
    pub tcp_attempt_fails: u32,
    pub tcp_estab_resets: u32,
    pub tcp_curr_estab: u32,
    pub tcp_in_segs: u32,
    pub tcp_out_segs: u32,
    pub tcp_retrans_segs: u32,
    pub tcp_in_errs: u32,
    pub tcp_out_rsts: u32,
    pub tcp_in_csum_errs: u32,
}

#[derive(Debug)]
pub struct Mib2UdpGroup {
    pub udp_in_datagrams: u32,
    pub udp_no_ports: u32,
    pub udp_in_errors: u32,
    pub udp_out_datagrams: u32,
    pub udp_rcvbuf_errors: u32,
    pub udp_sndbuf_errors: u32,
    pub udp_in_csum_errors: u32,
}

#[derive(Debug)]
pub enum CounterRecordData {
    IfCounters(IfCounters),
    EthernetCounters(EthernetCounters),
    HostDescription(HostDescription),
    HostAdapters(HostAdapters),
    HostCPU(HostCPU),
    HostDiskIO(HostDiskIO),
    HostNetIO(HostNetIO),
    HostMemory(HostMemory),
    Mib2IpGroup(Mib2IpGroup),
    Mib2IcmpGroup(Mib2IcmpGroup),
    Mib2TcpGroup(Mib2TcpGroup),
    Mib2UdpGroup(Mib2UdpGroup),
    Raw(Vec<u8>),
}

#[derive(Debug)]
pub struct CounterRecord {
    pub header: RecordHeader,
    pub data: CounterRecordData,
}

#[derive(Debug)]
pub enum Sample {
    Flow {
        header: SampleHeader,

        sampling_rate: u32,
        sample_pool: u32,
        drops: u32,
        input: u32,
        output: u32,
        flow_records_count: u32,
        records: Vec<FlowRecord>,
    },
    Counter {
        header: SampleHeader,

        counter_records_count: u32,
        records: Vec<CounterRecord>,
    },
    ExpandedFlow {
        header: SampleHeader,

        sampling_rate: u32,
        sample_pool: u32,
        drops: u32,
        input_if_format: u32,
        input_if_value: u32,
        output_if_format: u32,
        output_if_value: u32,
        flow_records_count: u32,
        records: Vec<FlowRecord>,
    },
    Drop {
        header: SampleHeader,

        drops: u32,
        input: u32,
        output: u32,
        reason: u32,
        flow_records_count: u32,
        records: Vec<FlowRecord>,
    },
}

#[derive(Debug)]
pub struct Datagram {
    pub version: u32,
    pub ip_version: u32,
    pub agent_ip: IpAddr,
    pub sub_agent_id: u32,
    pub sequence_number: u32,
    pub uptime: u32,
    pub samples_count: u32,

    pub samples: Vec<Sample>,
}

fn decode_string(buf: &mut Cursor<&[u8]>) -> Result<String, Error> {
    let len = buf.read_u32()?;
    let aligned_len = if len % 4 == 0 {
        len
    } else {
        len + (4 - len % 4)
    };

    let mut data = vec![0u8; aligned_len as usize];
    buf.read_exact(&mut data)?;
    data.truncate(len as usize);

    Ok(unsafe { String::from_utf8_unchecked(data) })
}

#[inline]
fn decode_mac(buf: &mut Cursor<&[u8]>) -> Result<[u8; 6], Error> {
    let mut data = [0u8; 6];

    buf.read_exact(&mut data)?;
    buf.advance(2); // aligned to 4

    Ok(data)
}

fn decode_counter_record(buf: &mut Cursor<&[u8]>) -> Result<CounterRecord, Error> {
    // read header first
    let data_format = buf.read_u32()?;
    let length = buf.read_u32()?;

    let data = match data_format {
        COUNTER_TYPE_IF => {
            let mut data = [0u8; size_of::<IfCounters>()];
            buf.read_exact(&mut data)?;
            CounterRecordData::IfCounters(unsafe {
                std::mem::transmute::<[u8; size_of::<IfCounters>()], IfCounters>(data)
            })
        }
        COUNTER_TYPE_ETH => {
            let mut data = [0u8; size_of::<EthernetCounters>()];
            buf.read_exact(&mut data)?;
            CounterRecordData::EthernetCounters(unsafe {
                std::mem::transmute::<[u8; size_of::<EthernetCounters>()], EthernetCounters>(data)
            })
        }

        COUNTER_TYPE_HOST_CPU => {
            let mut data = [0u8; size_of::<HostCPU>()];
            buf.read_exact(&mut data)?;
            CounterRecordData::HostCPU(unsafe {
                std::mem::transmute::<[u8; size_of::<HostCPU>()], HostCPU>(data)
            })
        }
        COUNTER_TYPE_HOST_DESCRIPTION => {
            let host = decode_string(buf)?;

            let mut uuid = [0u8; 16];
            buf.read_exact(&mut uuid)?;

            let machine_type = buf.read_u32()?;
            let os_name = buf.read_u32()?;

            let os_release = decode_string(buf)?;

            CounterRecordData::HostDescription(HostDescription {
                host,
                uuid,
                machine_type,
                os_name,
                os_release,
            })
        }
        COUNTER_TYPE_HOST_ADAPTERS => {
            let length = buf.read_u32()?;
            let mut adapters = Vec::with_capacity(length as usize);
            for _ in 0..length {
                let if_index = buf.read_u32()?;

                let count = buf.read_u32()?;
                let mut mac_addresses = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    mac_addresses.push(decode_mac(buf)?);
                }

                adapters.push(HostAdapter {
                    if_index,
                    mac_addresses,
                });
            }

            CounterRecordData::HostAdapters(HostAdapters { length, adapters })
        }
        COUNTER_TYPE_HOST_MEMORY => {
            let mut data = [0u8; size_of::<HostMemory>()];
            buf.read_exact(&mut data)?;
            CounterRecordData::HostMemory(unsafe {
                std::mem::transmute::<[u8; size_of::<HostMemory>()], HostMemory>(data)
            })
        }
        COUNTER_TYPE_HOST_DISK_IO => {
            let mut data = [0u8; size_of::<HostDiskIO>()];
            buf.read_exact(&mut data)?;
            CounterRecordData::HostDiskIO(unsafe {
                std::mem::transmute::<[u8; size_of::<HostDiskIO>()], HostDiskIO>(data)
            })
        }
        COUNTER_TYPE_HOST_NET_IO => {
            let mut data = [0u8; size_of::<HostNetIO>()];
            buf.read_exact(&mut data)?;
            CounterRecordData::HostNetIO(unsafe {
                std::mem::transmute::<[u8; size_of::<HostNetIO>()], HostNetIO>(data)
            })
        }

        COUNTER_TYPE_MIB2_IP_GROUP => {
            let mut data = [0u8; size_of::<Mib2IpGroup>()];
            buf.read_exact(&mut data)?;
            CounterRecordData::Mib2IpGroup(unsafe {
                std::mem::transmute::<[u8; size_of::<Mib2IpGroup>()], Mib2IpGroup>(data)
            })
        }
        COUNTER_TYPE_MIB2_ICMP_GROUP => {
            let mut data = [0u8; size_of::<Mib2IcmpGroup>()];
            buf.read_exact(&mut data)?;
            CounterRecordData::Mib2IcmpGroup(unsafe {
                std::mem::transmute::<[u8; size_of::<Mib2IcmpGroup>()], Mib2IcmpGroup>(data)
            })
        }
        COUNTER_TYPE_MIB2_TCP_GROUP => {
            let mut data = [0u8; size_of::<Mib2TcpGroup>()];
            buf.read_exact(&mut data)?;
            CounterRecordData::Mib2TcpGroup(unsafe {
                std::mem::transmute::<[u8; size_of::<Mib2TcpGroup>()], Mib2TcpGroup>(data)
            })
        }
        COUNTER_TYPE_MIB2_UDP_GROUP => {
            let mut data = [0u8; size_of::<Mib2UdpGroup>()];
            buf.read_exact(&mut data)?;
            CounterRecordData::Mib2UdpGroup(unsafe {
                std::mem::transmute::<[u8; size_of::<Mib2UdpGroup>()], Mib2UdpGroup>(data)
            })
        }
        _ => {
            let mut data = vec![0u8; length as usize];
            buf.read_exact(&mut data)?;
            CounterRecordData::Raw(data)
        }
    };

    Ok(CounterRecord {
        header: RecordHeader {
            data_format,
            length,
        },
        data,
    })
}

fn decode_flow_record(buf: &mut Cursor<&[u8]>) -> Result<FlowRecord, Error> {
    // read header first
    let data_format = buf.read_u32()?;
    let length = buf.read_u32()?;

    let record = match data_format {
        FLOW_TYPE_RAW => {
            let protocol = buf.read_u32()?;
            let frame_length = buf.read_u32()?;
            let stripped = buf.read_u32()?;
            let original_length = buf.read_u32()?;

            let mut header_data = vec![0; length as usize - 4 * 4];
            buf.read_exact(&mut header_data)?;

            FlowRecord::Raw(FlowRecordRaw {
                protocol,
                frame_length,
                stripped,
                original_length,
                header_data,
            })
        }
        FLOW_TYPE_ETH => {
            let mut data = [0u8; size_of::<FlowRecordSampleEthernet>()];
            buf.read_exact(&mut data)?;
            FlowRecord::SampledEthernet(unsafe {
                std::mem::transmute::<
                    [u8; size_of::<FlowRecordSampleEthernet>()],
                    FlowRecordSampleEthernet,
                >(data)
            })
        }
        FLOW_TYPE_IPV4 => {
            let mut data = [0u8; size_of::<SampledIpv4>()];
            buf.read_exact(&mut data)?;
            FlowRecord::SampledIpv4(unsafe {
                std::mem::transmute::<[u8; size_of::<SampledIpv4>()], SampledIpv4>(data)
            })
        }
        FLOW_TYPE_IPV6 => {
            let mut data = [0u8; size_of::<SampledIpv6>()];
            buf.read_exact(&mut data)?;
            FlowRecord::SampledIpv6(unsafe {
                std::mem::transmute::<[u8; size_of::<SampledIpv6>()], SampledIpv6>(data)
            })
        }
        FLOW_TYPE_EXT_SWITCH => {
            let mut data = [0u8; size_of::<ExtendedSwitch>()];
            buf.read_exact(&mut data)?;
            FlowRecord::ExtendedSwitch(unsafe {
                std::mem::transmute::<[u8; size_of::<ExtendedSwitch>()], ExtendedSwitch>(data)
            })
        }
        FLOW_TYPE_EXT_ROUTER => {
            let ip_version = buf.read_u32()?;
            let next_hop = if ip_version == 1 {
                let mut octets = [0u8; 4];
                buf.read_exact(&mut octets)?;
                IpAddr::from(octets)
            } else if ip_version == 2 {
                let mut octets = [0u8; 16];
                buf.read_exact(&mut octets)?;
                IpAddr::from(octets)
            } else {
                return Err(Error::UnknownIpVersion(ip_version));
            };

            let src_mask_len = buf.read_u32()?;
            let dst_mask_len = buf.read_u32()?;

            FlowRecord::ExtendedRouter(ExtendedRouter {
                next_hop_ip_version: ip_version,
                next_hop,
                src_mask_len,
                dst_mask_len,
            })
        }
        FLOW_TYPE_EXT_GATEWAY => {
            let ip_version = buf.read_u32()?;
            let next_hop = if ip_version == 1 {
                let mut octets = [0u8; 4];
                buf.read_exact(&mut octets)?;
                IpAddr::from(octets)
            } else if ip_version == 2 {
                let mut octets = [0u8; 16];
                buf.read_exact(&mut octets)?;
                IpAddr::from(octets)
            } else {
                return Err(Error::UnknownIpVersion(ip_version));
            };

            let r#as = buf.read_u32()?;
            let src_as = buf.read_u32()?;
            let src_peer_as = buf.read_u32()?;
            let as_destinations = buf.read_u32()?;

            let (as_path_type, as_path_length, as_path) = if as_destinations != 0 {
                let as_path_type = buf.read_u32()?;
                let as_path_length = buf.read_u32()?;

                // protection for as-path length
                if as_path_length > 1000 {
                    return Err(Error::TooManyAsPath);
                }
                if as_path_length > buf.remaining() as u32 - 4 {
                    return Err(Error::InvalidAsPathLength);
                }
                let mut as_path: Vec<u32> = Vec::with_capacity(as_path_length as usize);
                for _ in 0..as_path_length {
                    as_path.push(buf.read_u32()?);
                }

                (as_path_type, as_path_length, as_path)
            } else {
                (0, 0, vec![])
            };

            let communities_length = buf.read_u32()?;
            // protection for communities length
            if communities_length > 1000 {
                return Err(Error::TooManyCommunities);
            }
            if communities_length > buf.remaining() as u32 - 4 {
                return Err(Error::InvalidCommunitiesLength);
            }
            let mut communities = Vec::with_capacity(communities_length as usize);
            for _ in 0..communities_length {
                communities.push(buf.read_u32()?);
            }

            let local_pref = buf.read_u32()?;

            FlowRecord::ExtendedGateway(ExtendedGateway {
                next_hop_ip_version: ip_version,
                next_hop,
                r#as,
                src_as,
                src_peer_as,
                as_destinations,
                as_path_type,
                as_path_length,
                as_path,
                communities_length,
                communities,
                local_pref,
            })
        }
        FLOW_TYPE_EGRESS_QUEUE => FlowRecord::EgressQueue(EgressQueue {
            queue: buf.read_u32()?,
        }),
        FLOW_TYPE_EXT_ACL => {
            let number = buf.read_u32()?;
            let name = decode_string(buf)?;
            let direction = buf.read_u32()?;

            FlowRecord::ExtendedACL(ExtendedACL {
                number,
                name,
                direction,
            })
        }
        FLOW_TYPE_EXT_FUNCTION => {
            let symbol = decode_string(buf)?;

            FlowRecord::ExtendedFunction(ExtendedFunction { symbol })
        }
        _ => {
            // not support yet
            return Err(Error::UnsupportedFlowRecordType(data_format));
        }
    };

    Ok(record)
}

fn decode_sample(buf: &mut Cursor<&[u8]>) -> Result<Sample, Error> {
    // sample header
    let format = buf.read_u32()?;
    let length = buf.read_u32()?;
    let sample_sequence_number = buf.read_u32()?;
    #[allow(unused_assignments)]
    let mut source_id_type = 0;
    #[allow(unused_assignments)]
    let mut source_id_value = 0;

    match format {
        SAMPLE_FORMAT_FLOW | SAMPLE_FORMAT_COUNTER => {
            // Interlaced data-source format
            let source_id = buf.read_u32()?;

            source_id_type = source_id >> 24;
            source_id_value = source_id & 0x00FF_FFFF;
        }
        SAMPLE_FORMAT_EXPANDED_FLOW | SAMPLE_FORMAT_EXPANDED_COUNTER | SAMPLE_FORMAT_DROP => {
            // Explicit data-source format
            source_id_type = buf.read_u32()?;
            source_id_value = buf.read_u32()?;
        }
        _ => return Err(Error::UnknownSampleFormat(format)),
    }

    let sample = match format {
        SAMPLE_FORMAT_FLOW => {
            let sampling_rate = buf.read_u32()?;
            let sample_pool = buf.read_u32()?;
            let drops = buf.read_u32()?;
            let input = buf.read_u32()?;
            let output = buf.read_u32()?;
            let flow_records_count = buf.read_u32()?;

            if flow_records_count > 1000 {
                // protection against ddos
                return Err(Error::TooManyFlowRecords);
            }
            let mut records = Vec::with_capacity(flow_records_count as usize);
            for _ in 0..flow_records_count {
                records.push(decode_flow_record(buf)?);
            }

            Sample::Flow {
                header: SampleHeader {
                    format,
                    length,
                    sample_sequence_number,
                    source_id_type,
                    source_id_value,
                },
                sampling_rate,
                sample_pool,
                drops,
                input,
                output,
                flow_records_count,
                records,
            }
        }
        SAMPLE_FORMAT_COUNTER | SAMPLE_FORMAT_EXPANDED_COUNTER => {
            let counter_records_count = buf.read_u32()?;
            if counter_records_count > 1000 {
                return Err(Error::TooManyFlowRecords);
            }

            let mut records = Vec::with_capacity(counter_records_count as usize);
            for _ in 0..counter_records_count {
                records.push(decode_counter_record(buf)?);
            }

            Sample::Counter {
                header: SampleHeader {
                    format,
                    length,
                    sample_sequence_number,
                    source_id_type,
                    source_id_value,
                },
                counter_records_count,
                records,
            }
        }
        SAMPLE_FORMAT_EXPANDED_FLOW => {
            let sampling_rate = buf.read_u32()?;
            let sample_pool = buf.read_u32()?;
            let drops = buf.read_u32()?;
            let input_if_format = buf.read_u32()?;
            let input_if_value = buf.read_u32()?;
            let output_if_format = buf.read_u32()?;
            let output_if_value = buf.read_u32()?;

            let flow_records_count = buf.read_u32()?;
            if flow_records_count > 1000 {
                // protection against ddos
                return Err(Error::TooManyFlowRecords);
            }
            let mut records = Vec::with_capacity(flow_records_count as usize);
            for _ in 0..flow_records_count {
                records.push(decode_flow_record(buf)?);
            }

            Sample::ExpandedFlow {
                header: SampleHeader {
                    format,
                    length,
                    sample_sequence_number,
                    source_id_type,
                    source_id_value,
                },
                sampling_rate,
                sample_pool,
                drops,
                input_if_format,
                input_if_value,
                output_if_format,
                output_if_value,
                flow_records_count,
                records,
            }
        }
        SAMPLE_FORMAT_DROP => {
            let drops = buf.read_u32()?;
            let input = buf.read_u32()?;
            let output = buf.read_u32()?;
            let reason = buf.read_u32()?;

            let flow_records_count = buf.read_u32()?;
            if flow_records_count > 1000 {
                // protection against ddos
                return Err(Error::TooManyFlowRecords);
            }
            let mut records = Vec::with_capacity(flow_records_count as usize);
            for _ in 0..flow_records_count {
                records.push(decode_flow_record(buf)?);
            }

            Sample::Drop {
                header: SampleHeader {
                    format,
                    length,
                    sample_sequence_number,
                    source_id_type,
                    source_id_value,
                },
                drops,
                input,
                output,
                reason,
                flow_records_count,
                records,
            }
        }
        _ => {
            return Err(Error::UnknownSampleFormat(format));
        }
    };

    Ok(sample)
}

impl Datagram {
    pub fn decode(data: impl AsRef<[u8]>) -> Result<Datagram, Error> {
        let mut buf = Cursor::new(data.as_ref());
        let version = buf.read_u32()?;
        if version != 5 {
            return Err(Error::IncompatibleVersion);
        }

        let ip_version = buf.read_u32()?;
        let agent_ip = if ip_version == 1 {
            let mut octets = [0u8; 4];
            buf.read_exact(&mut octets)?;
            IpAddr::from(octets)
        } else if ip_version == 2 {
            let mut octets = [0u8; 16];
            buf.read_exact(&mut octets)?;
            IpAddr::from(octets)
        } else {
            return Err(Error::UnknownIpVersion(ip_version));
        };

        let sub_agent_id = buf.read_u32()?;
        let sequence_number = buf.read_u32()?;
        let uptime = buf.read_u32()?;
        let samples_count = buf.read_u32()?;

        if samples_count > 1000 {
            return Err(Error::TooManySamples);
        }

        let mut samples = Vec::with_capacity(samples_count as usize);
        for _ in 0..samples_count {
            samples.push(decode_sample(&mut buf)?);
        }

        Ok(Datagram {
            version,
            ip_version,
            agent_ip,
            sub_agent_id,
            sequence_number,
            uptime,
            samples_count,
            samples,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode() {
        let data = [
            0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01, 0xac, 0x10, 0x00, 0x11, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x00, 0x01, 0xaa, 0x67, 0xee, 0xaa, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x88, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00,
            0x04, 0x13, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x04, 0xaa, 0x00, 0x00, 0x04, 0x13, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x52,
            0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x4e, 0x00, 0xff, 0x12, 0x34, 0x35, 0x1b,
            0xff, 0xab, 0xcd, 0xef, 0xab, 0x64, 0x81, 0x00, 0x00, 0x20, 0x08, 0x00, 0x45, 0x00,
            0x00, 0x3c, 0x5c, 0x07, 0x00, 0x00, 0x7c, 0x01, 0x48, 0xa0, 0xac, 0x10, 0x20, 0xfe,
            0xac, 0x10, 0x20, 0xf1, 0x08, 0x00, 0x97, 0x61, 0xa9, 0x48, 0x0c, 0xb2, 0x61, 0x62,
            0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70,
            0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67,
            0x68, 0x69, 0x00, 0x00,
        ];

        Datagram::decode(data.as_ref()).unwrap();
    }

    #[test]
    fn decode_expanded() {
        let data = [
            0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01, 0x01, 0x02, 0x03, 0x04, 0x00, 0x00,
            0x00, 0x00, 0x0f, 0xa7, 0x72, 0xc2, 0x0f, 0x76, 0x73, 0x48, 0x00, 0x00, 0x00, 0x05,
            0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xdc, 0x20, 0x90, 0x93, 0x26, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x0f, 0x42, 0xa4, 0x00, 0x00, 0x3f, 0xff, 0x04, 0x38, 0xec, 0xda,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x42, 0xa4, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x0f, 0x42, 0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x03, 0xe9,
            0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x1e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x1e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x90,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0xea, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00,
            0x00, 0x80, 0x08, 0xec, 0xf5, 0x2a, 0x8f, 0xbe, 0x74, 0x83, 0xef, 0x30, 0x65, 0xb7,
            0x81, 0x00, 0x00, 0x1e, 0x08, 0x00, 0x45, 0x00, 0x05, 0xd4, 0x3b, 0xba, 0x40, 0x00,
            0x3f, 0x06, 0xbd, 0x99, 0xb9, 0x3b, 0xdc, 0x93, 0x58, 0xee, 0x4e, 0x13, 0x01, 0xbb,
            0xcf, 0xd6, 0x45, 0xb7, 0x1b, 0xc0, 0xd5, 0xb8, 0xff, 0x24, 0x80, 0x10, 0x00, 0x04,
            0x01, 0x55, 0x00, 0x00, 0x01, 0x01, 0x08, 0x0a, 0xc8, 0xc8, 0x56, 0x95, 0x00, 0x34,
            0xf6, 0x0f, 0xe8, 0x1d, 0xbd, 0x41, 0x45, 0x92, 0x4c, 0xc2, 0x71, 0xe0, 0xeb, 0x2e,
            0x35, 0x17, 0x7c, 0x2f, 0xb9, 0xa8, 0x05, 0x92, 0x0e, 0x03, 0x1b, 0x50, 0x53, 0x0c,
            0xe5, 0x7d, 0x86, 0x75, 0x32, 0x8a, 0xcc, 0xe2, 0x26, 0xa8, 0x90, 0x21, 0x78, 0xbf,
            0xce, 0x7a, 0xf8, 0xb5, 0x8d, 0x48, 0xe4, 0xaa, 0xfe, 0x26, 0x34, 0xe0, 0xad, 0xb9,
            0xec, 0x79, 0x74, 0xd8, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xdc, 0x20, 0x90,
            0x93, 0x27, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x42, 0xa4, 0x00, 0x00, 0x3f, 0xff,
            0x04, 0x39, 0x2c, 0xd9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f,
            0x42, 0xa4, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x42, 0x4b, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x00, 0x03, 0xe9, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x17, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x90, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0xca, 0x00, 0x00,
            0x00, 0x04, 0x00, 0x00, 0x00, 0x80, 0xda, 0xb1, 0x22, 0xfb, 0xd9, 0xcf, 0x74, 0x83,
            0xef, 0x30, 0x65, 0xb7, 0x81, 0x00, 0x00, 0x17, 0x08, 0x00, 0x45, 0x00, 0x05, 0xb4,
            0xe2, 0x28, 0x40, 0x00, 0x3f, 0x06, 0x15, 0x0f, 0xc3, 0xb5, 0xaf, 0x26, 0x05, 0x92,
            0xc6, 0x9e, 0x00, 0x50, 0x0f, 0xb3, 0x35, 0x8e, 0x36, 0x02, 0xa1, 0x01, 0xed, 0xb0,
            0x80, 0x10, 0x00, 0x3b, 0xf7, 0xd4, 0x00, 0x00, 0x01, 0x01, 0x08, 0x0a, 0xd2, 0xe8,
            0xac, 0xbe, 0x00, 0x36, 0xbc, 0x3c, 0x37, 0x36, 0xc4, 0x80, 0x3f, 0x66, 0x33, 0xc5,
            0x50, 0xa6, 0x63, 0xb2, 0x92, 0xc3, 0x6a, 0x7a, 0x80, 0x65, 0x0b, 0x22, 0x62, 0xfe,
            0x16, 0x9c, 0xab, 0x55, 0x03, 0x47, 0xa6, 0x54, 0x63, 0xa5, 0xbc, 0x17, 0x8e, 0x5a,
            0xf6, 0xbc, 0x24, 0x52, 0xe9, 0xd2, 0x7b, 0x08, 0xe8, 0xc2, 0x6b, 0x05, 0x1c, 0xc0,
            0x61, 0xb4, 0xe0, 0x43, 0x59, 0x62, 0xbf, 0x0a, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
            0x00, 0xdc, 0x04, 0x12, 0xa0, 0x65, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x42, 0xa8,
            0x00, 0x00, 0x3f, 0xff, 0xa4, 0x06, 0x9f, 0x9b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x0f, 0x42, 0xa8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x42, 0xa4,
            0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x03, 0xe9, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00,
            0x05, 0x39, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x39, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x90, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x05, 0xf2, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x80, 0x74, 0x83, 0xef, 0x30,
            0x65, 0xb7, 0x28, 0x99, 0x3a, 0x4e, 0x89, 0x27, 0x81, 0x00, 0x05, 0x39, 0x08, 0x00,
            0x45, 0x18, 0x05, 0xdc, 0x8e, 0x5c, 0x40, 0x00, 0x3a, 0x06, 0x53, 0x77, 0x89, 0x4a,
            0xcc, 0xd5, 0x59, 0xbb, 0xa9, 0x55, 0x07, 0x8f, 0xad, 0xdc, 0xf2, 0x9b, 0x09, 0xb4,
            0xce, 0x1d, 0xbc, 0xee, 0x80, 0x10, 0x75, 0x40, 0x58, 0x02, 0x00, 0x00, 0x01, 0x01,
            0x08, 0x0a, 0xb0, 0x18, 0x5b, 0x6f, 0xd7, 0xd6, 0x8b, 0x47, 0xee, 0x6a, 0x03, 0x0b,
            0x9b, 0x52, 0xb1, 0xca, 0x61, 0x4b, 0x84, 0x57, 0x75, 0xc4, 0xb2, 0x18, 0x11, 0x39,
            0xce, 0x5d, 0x2a, 0x38, 0x91, 0x29, 0x76, 0x11, 0x7d, 0xc1, 0xcc, 0x5c, 0x4b, 0x0a,
            0xde, 0xbb, 0xa8, 0xad, 0x9d, 0x88, 0x36, 0x8b, 0xc0, 0x02, 0x87, 0xa7, 0xa5, 0x1c,
            0xd9, 0x85, 0x71, 0x85, 0x68, 0x2b, 0x59, 0xc6, 0x2c, 0x3c, 0x84, 0x0c, 0x00, 0x00,
            0x00, 0x03, 0x00, 0x00, 0x00, 0xdc, 0x20, 0x90, 0x93, 0x28, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x0f, 0x42, 0xa4, 0x00, 0x00, 0x3f, 0xff, 0x04, 0x39, 0x6c, 0xd8, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x42, 0xa4, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x0f, 0x42, 0x4b, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x03, 0xe9, 0x00, 0x00,
            0x00, 0x10, 0x00, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x17,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x90, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x00, 0x05, 0xf2, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x80,
            0xda, 0xb1, 0x22, 0xfb, 0xd9, 0xcf, 0x74, 0x83, 0xef, 0x30, 0x65, 0xb7, 0x81, 0x00,
            0x00, 0x17, 0x08, 0x00, 0x45, 0x00, 0x05, 0xdc, 0x7e, 0x42, 0x40, 0x00, 0x3f, 0x06,
            0x12, 0x4d, 0xb9, 0x66, 0xdb, 0x43, 0x67, 0xc2, 0xa9, 0x20, 0x63, 0x75, 0x57, 0xae,
            0x6d, 0xbf, 0x59, 0x7c, 0x93, 0x71, 0x09, 0x67, 0x80, 0x10, 0x00, 0xeb, 0xfc, 0x16,
            0x00, 0x00, 0x01, 0x01, 0x08, 0x0a, 0x40, 0x96, 0x88, 0x38, 0x36, 0xe1, 0x64, 0xc7,
            0x1b, 0x43, 0xbc, 0x0e, 0x1f, 0x81, 0x6d, 0x39, 0xf6, 0x12, 0x0c, 0xea, 0xc0, 0xea,
            0x7b, 0xc1, 0x77, 0xe2, 0x92, 0x6a, 0xbf, 0xbe, 0x84, 0xd9, 0x00, 0x18, 0x57, 0x49,
            0x92, 0x72, 0x8f, 0xa3, 0x78, 0x45, 0x6f, 0xc6, 0x98, 0x8f, 0x71, 0xb0, 0xc5, 0x52,
            0x7d, 0x8a, 0x82, 0xef, 0x52, 0xdb, 0xe9, 0xdc, 0x0a, 0x52, 0xdb, 0x06, 0x51, 0x80,
            0x80, 0xa9, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xdc, 0x20, 0x90, 0x93, 0x29,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x42, 0xa4, 0x00, 0x00, 0x3f, 0xff, 0x04, 0x39,
            0xac, 0xd7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x42, 0xa4,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x42, 0xa5, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
            0x03, 0xe9, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x03, 0xbd, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x03, 0xbd, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x90, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0xf2, 0x00, 0x00, 0x00, 0x04,
            0x00, 0x00, 0x00, 0x80, 0x90, 0xe2, 0xba, 0x89, 0x21, 0xad, 0x74, 0x83, 0xef, 0x30,
            0x65, 0xb7, 0x81, 0x00, 0x03, 0xbd, 0x08, 0x00, 0x45, 0x00, 0x05, 0xdc, 0x76, 0xa2,
            0x40, 0x00, 0x38, 0x06, 0xac, 0x75, 0x33, 0x5b, 0x74, 0x6c, 0xc3, 0xb5, 0xae, 0x87,
            0x1f, 0x40, 0x80, 0x68, 0xab, 0xbb, 0x2f, 0x90, 0x01, 0xee, 0x3a, 0xaf, 0x80, 0x10,
            0x00, 0xeb, 0x8e, 0xf4, 0x00, 0x00, 0x01, 0x01, 0x08, 0x0a, 0x34, 0xc0, 0xff, 0x26,
            0xac, 0x90, 0xd5, 0xc4, 0xcc, 0xd7, 0xa4, 0xa5, 0x5b, 0xa3, 0x79, 0x33, 0xc1, 0x25,
            0xcd, 0x84, 0xdc, 0xaa, 0x37, 0xc9, 0xe3, 0xab, 0xc6, 0xb4, 0xeb, 0xe3, 0x8d, 0x72,
            0x06, 0xd1, 0x5a, 0x1f, 0x9a, 0x8b, 0xe9, 0x9a, 0xf7, 0x33, 0x35, 0xe5, 0xca, 0x67,
            0xba, 0x04, 0xf9, 0x3c, 0x27, 0xff, 0xa3, 0xca, 0x5e, 0x90, 0xf9, 0xc7, 0xd1, 0xe4,
            0xf8, 0xf5, 0x7a, 0x14, 0xdc, 0x1c, 0xb1, 0xde, 0x63, 0x75, 0xb2, 0x65, 0x27, 0xf0,
            0x0d, 0x29, 0xc5, 0x56, 0x60, 0x4a, 0x50, 0x10, 0x00, 0x77, 0xc0, 0xef, 0x00, 0x00,
            0x74, 0xcf, 0x8a, 0x79, 0x87, 0x77, 0x75, 0x64, 0x75, 0xeb, 0xa4, 0x56, 0xb4, 0xd8,
            0x70, 0xca, 0xe6, 0x11, 0xbb, 0x9f, 0xa1, 0x63, 0x95, 0xa1, 0xb4, 0x81, 0x8d, 0x50,
            0xe0, 0xd5, 0xa9, 0x2c, 0xd7, 0x8f, 0xfe, 0x78, 0xce, 0xff, 0x5a, 0xa6, 0xb6, 0xb9,
            0xf1, 0xe9, 0x5f, 0xda, 0xcb, 0xf3, 0x62, 0x61, 0x5f, 0x2b, 0x32, 0x95, 0x5d, 0x96,
            0x2e, 0xef, 0x32, 0x04, 0xff, 0xcc, 0x76, 0xba, 0x49, 0xab, 0x92, 0xa7, 0xf1, 0xcc,
            0x52, 0x68, 0xde, 0x94, 0x90, 0xdb, 0x1b, 0xa0, 0x28, 0x8a, 0xf8, 0x64, 0x55, 0x9c,
            0x9b, 0xf6, 0x9c, 0x44, 0xd9, 0x68, 0xc0, 0xe5, 0x2c, 0xe1, 0x3d, 0x29, 0x19, 0xef,
            0x8b, 0x0c, 0x9d, 0x0a, 0x7e, 0xcd, 0xc2, 0xe9, 0x85, 0x6b, 0x85, 0xb3, 0x97, 0xbe,
            0xc6, 0x26, 0xd2, 0xe5, 0x2e, 0x90, 0xa9, 0xac, 0xe3, 0xd8, 0xef, 0xbd, 0x7b, 0x40,
            0xf8, 0xb7, 0xe3, 0xc3, 0x8d, 0xa7, 0x38, 0x0f, 0x87, 0x7a, 0x50, 0x62, 0xc8, 0xb8,
            0xa4, 0x52, 0x6e, 0xdc, 0x92, 0x7f, 0xe6, 0x8d, 0x45, 0x39, 0xfd, 0x06, 0x6e, 0xd9,
            0xb5, 0x65, 0xac, 0xae, 0x2b, 0x8d, 0xea, 0xcf, 0xa2, 0x98, 0x0b, 0xc6, 0x43, 0x2e,
            0xa7, 0x71, 0x99, 0x2b, 0xea, 0xc3, 0x9c, 0x27, 0x74, 0x9e, 0xd5, 0x11, 0x60, 0x7a,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7b, 0xd6, 0x2a, 0x39, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];

        Datagram::decode(data).unwrap();
    }

    #[test]
    fn decode_drop_egress_queue() {
        let data = [
            0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01, 0xc0, 0xa8, 0x77, 0xb8, 0x00, 0x01,
            0x86, 0xa0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x30, 0x7e, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x2C, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x04, 0x0c, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x2a,
        ];

        let datagram = Datagram::decode(data).unwrap();
        assert_eq!(datagram.samples_count, 1);
        assert_eq!(datagram.samples.len(), 1);

        match datagram.samples.first().unwrap() {
            Sample::Drop {
                flow_records_count,
                records,
                ..
            } => {
                assert_eq!(*flow_records_count, 1);
                assert_eq!(records.len(), 1);

                assert!(
                    matches!(records[0], FlowRecord::EgressQueue(EgressQueue { queue }) if queue == 42)
                )
            }
            _ => panic!(),
        }
    }

    #[test]
    fn decode_drop_extended_acl() {
        let data = [
            0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01, 0xc0, 0xa8, 0x77, 0xb8, 0x00, 0x01,
            0x86, 0xa0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x30, 0x7e, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x04, 0x0d, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x2a, 0x00, 0x00, 0x00, 0x04,
            0x66, 0x6f, 0x6f, 0x21, 0x00, 0x00, 0x00, 0x02,
        ];

        let datagram = Datagram::decode(data).unwrap();
        assert_eq!(datagram.samples_count, 1);
        assert_eq!(datagram.samples.len(), 1);

        match datagram.samples.first().unwrap() {
            Sample::Drop {
                flow_records_count,
                records,
                ..
            } => {
                assert_eq!(*flow_records_count, 1);
                assert_eq!(records.len(), 1);

                assert!(
                    matches!(&records[0], FlowRecord::ExtendedACL(ExtendedACL {number,name,direction}) if *number == 42 && name == "foo!" && *direction == 2 )
                )
            }
            _ => panic!(),
        }
    }

    #[test]
    fn decode_drop_extended_function() {
        let data = [
            0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01, 0xc0, 0xa8, 0x77, 0xb8, 0x00, 0x01,
            0x86, 0xa0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x30, 0x7e, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x32, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x04, 0x0e, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x06, 0x66, 0x6f, 0x6f, 0x62,
            0x61, 0x72,
        ];

        let datagram = Datagram::decode(data).unwrap();
        assert_eq!(datagram.samples_count, 1);
        assert_eq!(datagram.samples.len(), 1);

        match datagram.samples.first().unwrap() {
            Sample::Drop {
                records,
                flow_records_count,
                ..
            } => {
                assert_eq!(*flow_records_count, 1);
                assert_eq!(records.len(), 1);

                assert!(
                    matches!(&records[0], FlowRecord::ExtendedFunction(ExtendedFunction { symbol }) if symbol == "foobar")
                )
            }
            _ => panic!(),
        }
    }

    #[test]
    fn decode_hsflowd() {
        let data = [
            0, 0, 0, 5, 0, 0, 0, 1, 192, 168, 88, 254, 0, 1, 134, 161, 0, 0, 0, 193, 0, 88, 14, 97,
            0, 0, 0, 1, 0, 0, 0, 4, 0, 0, 2, 176, 0, 0, 0, 193, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0,
            10, 0, 0, 7, 209, 0, 0, 0, 20, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 4, 217, 245, 249,
            228, 34, 0, 0, 0, 0, 7, 218, 0, 0, 0, 28, 0, 34, 72, 241, 0, 0, 5, 233, 0, 0, 0, 0, 0,
            19, 134, 52, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 217, 0, 0, 0, 60, 0, 0, 0, 1,
            0, 0, 0, 200, 0, 1, 212, 192, 255, 255, 255, 255, 0, 1, 29, 89, 0, 0, 33, 152, 0, 0,
            158, 194, 0, 0, 17, 199, 0, 0, 0, 69, 0, 159, 128, 136, 1, 188, 82, 123, 0, 2, 29, 197,
            0, 0, 0, 55, 0, 0, 20, 132, 0, 0, 0, 0, 0, 0, 7, 216, 0, 0, 0, 100, 0, 0, 10, 21, 0, 0,
            0, 48, 0, 0, 0, 0, 0, 0, 10, 9, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 41, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 198, 0, 0, 5, 35, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 7, 215, 0, 0, 0, 76, 0, 0, 0, 2, 0, 0, 0, 64, 0, 165,
            101, 64, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 165, 101, 39,
            0, 127, 188, 144, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 213, 0, 0, 0, 52, 0, 0, 0, 236, 146,
            155, 176, 0, 0, 0, 0, 137, 252, 218, 48, 0, 0, 0, 17, 5, 0, 23, 69, 6, 0, 0, 0, 16, 50,
            215, 60, 0, 0, 47, 173, 141, 0, 136, 93, 171, 0, 0, 0, 46, 189, 129, 120, 0, 6, 248,
            169, 74, 0, 0, 7, 212, 0, 0, 0, 72, 0, 0, 0, 15, 172, 23, 96, 0, 0, 0, 0, 0, 243, 158,
            128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 34, 48, 0, 0, 0, 0, 9, 230, 115, 208, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 237, 30, 75, 6, 5, 105, 101, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 7, 211, 0, 0, 0, 80, 64, 24, 81, 236, 64, 40, 81, 236, 64, 49,
            235, 133, 0, 0, 0, 0, 0, 0, 16, 207, 0, 0, 0, 32, 0, 0, 16, 84, 0, 0, 140, 106, 3, 47,
            183, 232, 0, 1, 5, 74, 0, 141, 79, 250, 64, 93, 32, 206, 0, 19, 227, 4, 0, 38, 105, 62,
            0, 18, 49, 18, 61, 30, 193, 151, 73, 87, 30, 174, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 7, 214, 0, 0, 0, 40, 0, 0, 0, 0, 219, 76, 237, 124, 0, 47, 243, 78, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 1, 105, 246, 42, 50, 0, 70, 51, 25, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            7, 208, 0, 0, 0, 64, 0, 0, 0, 6, 102, 101, 100, 111, 114, 97, 0, 0, 26, 163, 85, 64,
            167, 93, 120, 125, 152, 156, 4, 217, 245, 249, 228, 34, 0, 0, 0, 3, 0, 0, 0, 2, 0, 0,
            0, 23, 54, 46, 49, 49, 46, 49, 48, 45, 51, 48, 48, 46, 102, 99, 52, 49, 46, 120, 56,
            54, 95, 54, 52, 0,
        ];

        let datagram = Datagram::decode(data).unwrap();

        assert_eq!(datagram.samples_count, 1);

        println!("{:#?}", datagram);
    }

    #[test]
    fn sizes() {
        assert_eq!(size_of::<HostCPU>(), 80);
        assert_eq!(size_of::<HostMemory>(), 72);
        assert_eq!(size_of::<HostDiskIO>(), 52);
        assert_eq!(size_of::<HostNetIO>(), 40);
        // assert_eq!(size_of::<HostDescription>(), 64);

        assert_eq!(size_of::<Mib2IpGroup>(), 76);
        assert_eq!(size_of::<Mib2IcmpGroup>(), 100);
        assert_eq!(size_of::<Mib2TcpGroup>(), 60);
        assert_eq!(size_of::<Mib2UdpGroup>(), 28);
    }
}
