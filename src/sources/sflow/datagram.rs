//!
//! # sFlow
//! sFlow is a robust, extensible protocol for reporting performance and system counters, as well as network flows. From the [InMon Corporation website](http://www.inmon.com/technology/):
//!
//! > Originally developed by InMon, sFlow is the leading, multi-vendor, standard for monitoring high-speed switched and routed networks. sFlow technology is built into network equipment and gives complete visibility into network activity, enabling effective management and control of network resources. InMon is a founding member of the sFlow.org industry consortium.
//!
//! See the InMon [Network Equipment page](http://www.sflow.org/products/network.php) for a list of platforms and devices that support sFlow.
//!
//! By bringing together both flow data and performance counter data, it's possible to get a wider and more holistic view of overall network and system performance. It's important to understand how the sFlow protocol and its structures work so you can effectively ingest and parse sFlow data.
//!
//! 1. [Structures](#structures)
//! 2. [Samples](#samples)
//! 1. [Flow Sample](#flow-sample)
//! 2. [Counter Sample](#counter-sample)
//! 3. [Expanded Flow Sample](#expanded-flow-sample)
//! 4. [Expanded Counter Sample](#expanded-counter-sample)
//! 3. [Flow Data](#flow-data)
//! 4. [Counter Data](#counter-data)
//! 5. [Attributions](#attributions)
//!
//! # Structures
//! sFlow structures define specific data sets that follow a defined standard. The Flow Analyzer currently supports most of the standard sFlow-defined structures. Vendors and open source developers are free to define and use their own structures, but support for those structures (especially proprietary, vendor-specific structures) is limited in this project.
//!
//! A list of the standard, sFlow-defined structures can be found [on the sFlow.org website](http://www.sflow.org/developers/structures.php).
//!
//! # Samples
//! The top four structures help define the layout and type of the structures beneat them. Each of these samples tells the collector what type of records are contained inside, as well as the sFlow Agent's IP address, Agent ID, the sequence number, and more. This gives us the "lay of the land" while parsing through the records at a lower level.
//!
//! The four top sample types are as follows:
//! Type | Enterprise | Format | Structure Name | Link |
//! --          | - | -  | --                           | -- |
//! Sample      | 0 | 1  | Flow Sample                  | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Sample      | 0 | 2  | Counter Sample               | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Sample      | 0 | 3  | Expanded Flow Sample         | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Sample      | 0 | 4  | Expanded Counter Sample      | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//!
//! The **Enterprise** number defines the vendor or developer whose product is exporting information. sFlow protocol developer inMon Corporation is enterprise number zero (0). Broadcom is enterprise number 4413 and Nvidia is enterprise number 5703, just to give two other examples.
//!
//! The **Format** number defines specific data structures used by the vendor. For example, [ _Enterprise, Format_ ] numbers [_0, 1006_] are defined as the "Extended MPLS" structure by inMon Corporation. Another example would be [ _0, 2101_ ] which is defined as the "Virtual CPU Counter" structure.
//!
//! When the Enterprise and Format numbers are combined we know what data structure has been sent, and by referencing that defined structure we can parse out the data.
//!
//! ## Flow Sample
//! Flow Samples [ _0, 1_ ] are pretty much what you'd think they would be if you're familiar with Netflow or IPFIX. This mirrors a lot of the same functionality of Netflow v5, Netflow v9, and IPFIX (aka Netflow v10). Flow samples can include source and destination IP addresses, port numbers, protocols, and packet headers.
//!
//! The sFlow protocol then goes quite a bit beyond the typical network flow protocols by reporting application information such as HTTP transactions, NFS storage transactions, NAT, Fibre Channel, and more. This makes sFlow a good protocol for monitoring network flows, and also marrying that information with application-level flows.
//!
//! ## Counter Sample
//! Counter Samples [ _0, 2_ ] provide numeric information about systems and system performance. Examples of counter information include:
//! - Overall CPU count
//! - Free memory
//! - Dropped packets
//! - Bytes out
//! - Packets out
//! - Errors
//!
//! By combining counter information with flow data we can present a wider, more holistic picture of an organization's systems and their performance over time.
//!
//! ## Expanded Flow Sample
//! The Expanded Flow Sample does what [Flow Samples](#flow-samples) do, but they allow for the use of ifIndex numeric values over 2^24. From the sFlow v5 definition:
//!
//! > The expanded encodings are provided to support the maximum possible values for ifIndex, even though large ifIndex values are not encouraged.
//! >
//! > --<cite>[SFLOW-DATAGRAM5 Documentation File](http://sflow.org/SFLOW-DATAGRAM5.txt)</cite>
//!
//! ## Expanded Counter Sample
//! The Expanded Counter Sample does for [Counter Samples](#counter-samples) what [Expanded Flow Samples](#expanded-flow-samples) do for regular [Flow Samples](#flow-samples). As networks and systems become larger and faster it's important that protocols can handle very large values.
//!
//! # Flow Data
//! The default structures for flow data are shown below:
//!
//! Type | Enterprise | Format | Name | Supported | Link |
//! ---     | --- | --- | ---                               | ---           | --- |
//! Flow    | 0 | 1     | Raw Packet Header                 | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 2     | Ethernet Frame Data               | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 3     | Packet IPv4 Data                  | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 4     | Packet IPv6 Data                  | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 1001  | Extended Switch                   | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 1002  | Extended Router                   | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 1003  | Extended Gateway                  | In Progress   | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 1004  | Extended User                     | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 1005  | Extended URL (deprecated)         | N/A           | N/A |
//! Flow    | 0 | 1006  | Extended MPLS                     | In Progress   | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 1007  | Extended NAT                      | In Progress   | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 1008  | Extended MPLS Tunnel              | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 1009  | Extended MPLS VC                  | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 1010  | Extended MPLS FTN                 | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 1011  | Extended MPLS LDP FEC             | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 1012  | Extended VLAN Tunnel              | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Flow    | 0 | 1013  | Extended 802.11 Payload           | In Progress   | [sFlow 802.11 Structures](http://www.sflow.org/sflow_80211.txt) |
//! Flow    | 0 | 1014  | Extended 802.11 RX                | Yes           | [sFlow 802.11 Structures](http://www.sflow.org/sflow_80211.txt) |
//! Flow    | 0 | 1015  | Extended 802.11 TX                | Yes           | [sFlow 802.11 Structures](http://www.sflow.org/sflow_80211.txt) |
//! Flow    | 0 | 1016  | Extended 802.11 Aggregation       | In Progress   | [sFlow 802.11 Structures](http://www.sflow.org/sflow_80211.txt) |
//! Flow    | 0 | 1017  | Extended OpenFlow v1 (deprecated) | N/A           | N/A |
//! Flow    | 0 | 1018  | Extended Fibre Channel            | In Progress   | [sFlow, CEE and FCoE](http://sflow.org/discussion/sflow-discussion/0244.html) |
//! Flow    | 0 | 1019  | Extended Queue Length             | In Progress   | [sFlow for queue length monitoring](https://groups.google.com/forum/#!topic/sflow/dz0nsXqBYAw) |
//! Flow    | 0 | 1020  | Extended NAT Port                 | In Progress   | [sFlow Port NAT Structure](http://www.sflow.org/sflow_pnat.txt) |
//! Flow    | 0 | 1021  | Extended L2 Tunnel Egress         | In Progress   | [sFlow Tunnel Structure](http://www.sflow.org/sflow_tunnels.txt) |
//! Flow    | 0 | 1022  | Extended L2 Tunnel Ingress        | In Progress   | [sFlow Tunnel Structure](http://www.sflow.org/sflow_tunnels.txt) |
//! Flow    | 0 | 1023  | Extended IPv4 Tunnel Egress       | In Progress   | [sFlow Tunnel Structure](http://www.sflow.org/sflow_tunnels.txt) |
//! Flow    | 0 | 1024  | Extended IPv4 Tunnel Ingress      | In Progress   | [sFlow Tunnel Structure](http://www.sflow.org/sflow_tunnels.txt) |
//! Flow    | 0 | 1025  | Extended IPv6 Tunnel Egress       | In Progress   | [sFlow Tunnel Structure](http://www.sflow.org/sflow_tunnels.txt) |
//! Flow    | 0 | 1026  | Extended IPv6 Tunnel Ingress      | In Progress   | [sFlow Tunnel Structure](http://www.sflow.org/sflow_tunnels.txt) |
//! Flow    | 0 | 1027  | Extended Decapsulate Egress       | In Progress   | [sFlow Tunnel Structure](http://www.sflow.org/sflow_tunnels.txt) |
//! Flow    | 0 | 1028  | Extended Decapsulate Ingress      | In Progress   | [sFlow Tunnel Structure](http://www.sflow.org/sflow_tunnels.txt) |
//! Flow    | 0 | 1029  | Extended VNI Egress               | In Progress   | [sFlow Tunnel Structure](http://www.sflow.org/sflow_tunnels.txt) |
//! Flow    | 0 | 1030  | Extended VNI Ingress              | In Progress   | [sFlow Tunnel Structure](http://www.sflow.org/sflow_tunnels.txt) |
//! Flow    | 0 | 1031  | Extended InfiniBand LRH           | Yes           | [sFlow InfiniBand Structures](http://sflow.org/draft_sflow_infiniband_2.txt) |
//! Flow    | 0 | 1032  | Extended InfiniBand GRH           | In Progress   | [sFlow InfiniBand Structures](http://sflow.org/draft_sflow_infiniband_2.txt) |
//! Flow    | 0 | 1033  | Extended InfiniBand BRH           | Yes           | [sFlow InfiniBand Structures](http://sflow.org/draft_sflow_infiniband_2.txt) |
//! Flow    | 0 | 2000  | Transaction                       | Yes           | [Host Performance Statistics Thread, Peter Phaal](http://www.sflow.org/discussion/sflow-discussion/0282.html) |
//! Flow    | 0 | 2001  | Extended NFS Storage Transaction  | Yes           | [Host Performance Statistics Thread, Peter Phaal](http://www.sflow.org/discussion/sflow-discussion/0282.html) |
//! Flow    | 0 | 2002  | Extended SCSI Storage Transaction | Yes           | [Host Performance Statistics Thread, Peter Phaal](http://www.sflow.org/discussion/sflow-discussion/0282.html) |
//! Flow    | 0 | 2003  | Extended Web Transaction          | Yes           | [Host Performance Statistics Thread, Peter Phaal](http://www.sflow.org/discussion/sflow-discussion/0282.html) |
//! Flow    | 0 | 2100  | Extended Socket IPv4              | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Flow    | 0 | 2101  | Extended Socket IPv6              | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Flow    | 0 | 2102  | Extended Proxy Socket IPv4        | In Progress   | [sFlow HTTP Structures](http://www.sflow.org/sflow_http.txt) |
//! Flow    | 0 | 2103  | Extended Proxy Socket IPv6        | In Progress   | [sFlow HTTP Structures](http://www.sflow.org/sflow_http.txt) |
//! Flow    | 0 | 2200  | Memcached Operation               | In Progress   | [sFlow Memcache Structures](http://www.sflow.org/sflow_memcache.txt) |
//! Flow    | 0 | 2201  | HTTP Request (deprecated)         | N/A           | N/A |
//! Flow    | 0 | 2202  | App Operation                     | In Progress   | [sFlow Application Structures](http://www.sflow.org/sflow_application.txt) |
//! Flow    | 0 | 2203  | App Parent Context                | In Progress   | [sFlow Application Structures](http://www.sflow.org/sflow_application.txt) |
//! Flow    | 0 | 2204  | App Initiator                     | In Progress   | [sFlow Application Structures](http://www.sflow.org/sflow_application.txt) |
//! Flow    | 0 | 2205  | App Target                        | In Progress   | [sFlow Application Structures](http://www.sflow.org/sflow_application.txt) |
//! Flow    | 0 | 2206  | HTTP Request                      | Yes           | [sFlow HTTP Structures](http://www.sflow.org/sflow_http.txt) |
//! Flow    | 0 | 2207  | Extended Proxy Request            | In Progress   | [sFlow HTTP Structures](http://www.sflow.org/sflow_http.txt) |
//! Flow    | 0 | 2208  | Extended Nav Timing               | Yes           | [Navigation Timing Thread](https://groups.google.com/forum/?fromgroups#!topic/sflow/FKzkvig32Tk) |
//! Flow    | 0 | 2209  | Extended TCP Info                 | Yes           | [sFlow Google Group, Peter Phaal](https://groups.google.com/forum/#!topic/sflow/JCG9iwacLZA) |
//!
//! # Counter Data
//! The default structures for counter data are shown below:
//!
//! Type        | Enterprise | Format | Name                                        | Supported     | Link |
//! ---         | ---   | ---   | ---                                               | ---           | --- |
//! Counter     | 0     | 1     | Generic Interface Counters                        | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Counter     | 0     | 2     | Ethernet Interface Counters                       | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Counter     | 0     | 3     | Token Ring Counters                               | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Counter     | 0     | 4     | 100 BaseVG Interface Counters                     | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Counter     | 0     | 5     | VLAN Counters                                     | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Counter     | 0     | 6     | 802.11 Counters                                   | Yes           | [sFlow 802.11 Structures](http://www.sflow.org/sflow_80211.txt) |
//! Counter     | 0     | 7     | LAG Port Statistics                               | Yes           | [sFlow LAG Port Statistics](http://www.sflow.org/sflow_lag.txt) |
//! Counter     | 0     | 8     | Slow Path Counts                                  | Yes           | [Slow Path Counters](https://groups.google.com/forum/#!topic/sflow/4JM1_Mmoz7w) |
//! Counter     | 0     | 9     | InfiniBand Counters                               | Yes           | [sFlow InfiniBand Structures](http://sflow.org/draft_sflow_infiniband_2.txt) |
//! Counter     | 0     | 10    | Optical SFP / QSFP Counters                       | Yes           | [sFlow Optical Interface Structures](http://www.sflow.org/sflow_optics.txt) |
//! Counter     | 0     | 1001  | Processor                                         | Yes           | [sFlow Version 5](http://sflow.org/sflow_version_5.txt) |
//! Counter     | 0     | 1002  | Radio Utilization                                 | Yes           | [sFlow 802.11 Structures](http://www.sflow.org/sflow_80211.txt) |
//! Counter     | 0     | 1003  | Queue Length                                      | In Progress   | [sFlow Queue Length Histogram Counters](https://groups.google.com/forum/#!searchin/sflow/format$20$3D/sflow/dz0nsXqBYAw/rFOuMcLYjmkJ) |
//! Counter     | 0     | 1004  | OpenFlow Port                                     | In Progress   | [sFlow OpenFlow Structures](http://www.sflow.org/sflow_openflow.txt) |
//! Counter     | 0     | 1005  | OpenFlow Port Name                                | In Progress   | [sFlow OpenFlow Structures](http://www.sflow.org/sflow_openflow.txt) |
//! Counter     | 0     | 2000  | Host Description                                  | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Counter     | 0     | 2001  | Host Adapters                                     | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Counter     | 0     | 2002  | Host Parent                                       | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Counter     | 0     | 2003  | Host CPU                                          | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Counter     | 0     | 2004  | Host Memory                                       | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Counter     | 0     | 2005  | Host Disk I/O                                     | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Counter     | 0     | 2006  | Host Network I/O                                  | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Counter     | 0     | 2007  | MIB2 IP Group                                     | Yes           | [sFlow Host TCP/IP Counters](http://www.sflow.org/sflow_host_ip.txt) |
//! Counter     | 0     | 2008  | MIB2 ICMP Group                                   | Yes           | [sFlow Host TCP/IP Counters](http://www.sflow.org/sflow_host_ip.txt) |
//! Counter     | 0     | 2009  | MIB2 TCP Group                                    | Yes           | [sFlow Host TCP/IP Counters](http://www.sflow.org/sflow_host_ip.txt) |
//! Counter     | 0     | 2010  | MIB2 UDP Group                                    | Yes           | [sFlow Host TCP/IP Counters](http://www.sflow.org/sflow_host_ip.txt) |
//! Counter     | 0     | 2100  | Virtual Node                                      | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Counter     | 0     | 2101  | Virtual CPU                                       | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Counter     | 0     | 2102  | Virtual Memory                                    | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Counter     | 0     | 2103  | Virtual Disk I/O                                  | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Counter     | 0     | 2104  | Virtual Network I/O                               | Yes           | [sFlow Host Structures](http://www.sflow.org/sflow_host.txt) |
//! Counter     | 0     | 2105  | JMX Runtime                                       | Yes           | [sFlow Java Virtual Machine Structures](http://www.sflow.org/sflow_jvm.txt) |
//! Counter     | 0     | 2106  | JMX Statistics                                    | Yes           | [sFlow Java Virtual Machine Structures](http://www.sflow.org/sflow_jvm.txt) |
//! Counter     | 0     | 2200  | Memcached Counters (deprecated)                   | N/A           | N/A |
//! Counter     | 0     | 2201  | HTTP Counters                                     | In Progress   | [sFlow HTTP Structures](http://www.sflow.org/sflow_http.txt) |
//! Counter     | 0     | 2202  | App Operations                                    | In Progress   | [sFlow Application Structures](http://www.sflow.org/sflow_application.txt) |
//! Counter     | 0     | 2203  | App Resources                                     | In Progress   | [sFlow Application Structures](http://www.sflow.org/sflow_application.txt) |
//! Counter     | 0     | 2204  | Memcache Counters                                 | In Progress   | [sFlow Memcache Structures](http://www.sflow.org/sflow_memcache.txt) |
//! Counter     | 0     | 2206  | App Workers                                       | In Progress   | [sFlow Application Structures](http://www.sflow.org/sflow_application.txt) |
//! Counter     | 0     | 2207  | OVS DP Statistics                                 | In Progress   | -- |
//! Counter     | 0     | 3000  | Energy                                            | Yes           | [Energy Management Thread](https://groups.google.com/forum/#!topic/sflow/gN3nxSi2SBs) |
//! Counter     | 0     | 3001  | Temperature                                       | Yes           | [Energy Management Thread](https://groups.google.com/forum/#!topic/sflow/gN3nxSi2SBs) |
//! Counter     | 0     | 3002  | Humidity                                          | Yes           | [Energy Management Thread](https://groups.google.com/forum/#!topic/sflow/gN3nxSi2SBs) |
//! Counter     | 0     | 3003  | Fans                                              | Yes           | [Energy Management Thread](https://groups.google.com/forum/#!topic/sflow/gN3nxSi2SBs) |
//! Counter     | 4413  | 1     | Broadcom Switch Device Buffer Utilization         | Yes           | [sFlow Broadcom Switch ASIC Table Utilization Structures](http://www.sflow.org/sflow_broadcom_tables.txt) |
//! Counter     | 4413  | 2     | Broadcom Switch Port Level Buffer Utilization     | Yes           | [sFlow Broadcom Switch ASIC Table Utilization Structures](http://www.sflow.org/sflow_broadcom_tables.txt) |
//! Counter     | 4413  | 3     | Broadcom Switch ASIC Hardware Table Utilization   | Yes           | [sFlow Broadcom Switch ASIC Table Utilization Structures](http://www.sflow.org/sflow_broadcom_tables.txt) |
//! Counter     | 5703  | 1     | NVIDIA GPU Statistics                             | Yes           | [sFlow NVML GPU Structure](http://www.sflow.org/sflow_nvml.txt) |
//!

#![allow(dead_code)]

use std::io::{Cursor, Read};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use bytes::Buf;

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
const FLOW_TYPE_EXT_LINUX_REASON: u32 = 1042;
const FLOW_TYPE_EXT_TCP_INFO: u32 = 2209;

// Opaque counter_data types according to https://sflow.org/SFLOW-STRUCTS5.txt
const COUNTER_TYPE_INTERFACE: u32 = 1;
const COUNTER_TYPE_ETHERNET: u32 = 2;
const COUNTER_TYPE_TOKEN_RING: u32 = 3;
const COUNTER_TYPE_VG: u32 = 4;
const COUNTER_TYPE_VLAN: u32 = 5;
const COUNTER_TYPE_80211: u32 = 6;
const COUNTER_TYPE_LACP: u32 = 7;
const COUNTER_TYPE_SLOW_PATH: u32 = 8;
const COUNTER_TYPE_INFINIBAND: u32 = 9;
const COUNTER_TYPE_SFP: u32 = 10;
const COUNTER_TYPE_PROCESSOR: u32 = 1001;
const COUNTER_TYPE_PORT_NAME: u32 = 1005;
const COUNTER_TYPE_HOST_DESCRIPTION: u32 = 2000;
const COUNTER_TYPE_HOST_ADAPTERS: u32 = 2001;
const COUNTER_TYPE_HOST_PARENT: u32 = 2002;
const COUNTER_TYPE_HOST_CPU: u32 = 2003;
const COUNTER_TYPE_HOST_MEMORY: u32 = 2004;
const COUNTER_TYPE_HOST_DISK_IO: u32 = 2005;
const COUNTER_TYPE_HOST_NET_IO: u32 = 2006;
const COUNTER_TYPE_MIB2_IP_GROUP: u32 = 2007;
const COUNTER_TYPE_MIB2_ICMP_GROUP: u32 = 2008;
const COUNTER_TYPE_MIB2_TCP_GROUP: u32 = 2009;
const COUNTER_TYPE_MIB2_UDP_GROUP: u32 = 2010;
const COUNTER_TYPE_VIRT_NODE: u32 = 2100;
const COUNTER_TYPE_VIRT_CPU: u32 = 2101;
const COUNTER_TYPE_VIRT_MEMORY: u32 = 2102;
const COUNTER_TYPE_VIRT_DISK_IO: u32 = 2103;
const COUNTER_TYPE_VIRT_NET_IO: u32 = 2104;
const COUNTER_TYPE_ENERGY: u32 = 3000;
const COUNTER_TYPE_TEMPERATURE: u32 = 3001;
const COUNTER_TYPE_HUMIDITY: u32 = 3002;
const COUNTER_TYPE_FANS: u32 = 3003;
const COUNTER_TYPE_NVIDIA_GPU: u32 = (5703 << 12) + 1;
const COUNTER_TYPE_BCM_TABLES: u32 = (4413 << 12) + 3;

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

impl From<bytes::TryGetError> for Error {
    fn from(err: bytes::TryGetError) -> Error {
        Error::Io(err.into())
    }
}

#[derive(Debug)]
pub struct SampleHeader {
    pub format: u32,
    pub length: u32,

    pub sample_sequence_number: u32,
    pub source_id_type: u32,
    pub source_id_value: u32,
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
    pub header_bytes: Vec<u8>,
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
    pub src_ip: Ipv6Addr,
    pub dst_ip: Ipv6Addr,
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
    pub next_hop: IpAddr,
    pub src_mask_len: u32,
    pub dst_mask_len: u32,
}

#[derive(Debug)]
pub struct ExtendedGateway {
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
pub struct ExtendedTCPInfo {
    pub direction: u32,  /* Sampled packet direction */
    pub snd_mss: u32,    /* Cached effective mss, not including SACKS */
    pub rcv_mss: u32,    /* Max. recv. segment size */
    pub unacked: u32,    /* Packets which are "in flight" */
    pub lost: u32,       /* Lost packets */
    pub retrans: u32,    /* Retransmitted packets */
    pub pmtu: u32,       /* Last pmtu seen by socket */
    pub rtt: u32,        /* smoothed RTT (microseconds) */
    pub rttvar: u32,     /* RTT variance (microseconds) */
    pub snd_cwnd: u32,   /* Sending congestion window */
    pub reordering: u32, /* Reordering */
    pub min_rtt: u32,    /* Minimum RTT (microseconds) */
}

// Linux drop_monitor reason
// opaque = flow_data; enterprise = 0; format = 1042
// https://github.com/torvalds/linux/blob/master/include/net/dropreason.h
// XDR spec:
//  struct extended_linux_drop_reason {
//    string reason<>; /* NET_DM_ATTR_REASON */
//  }
#[derive(Debug)]
pub struct ExtendedLinuxReason {
    pub reason: String,
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
    ExtendedTCPInfo(ExtendedTCPInfo),
    ExtendedLinuxReason(ExtendedLinuxReason),
}

#[derive(Debug)]
pub struct IfCounters {
    pub index: u32,
    pub typ: u32,
    pub speed: u64,
    pub direction: u32,
    pub status: u32,
    pub in_octets: u64,
    pub in_ucast_pkts: u32,
    pub in_multicast_pkts: u32,
    pub in_broadcast_pkts: u32,
    pub in_discards: u32,
    pub in_errors: u32,
    pub in_unknown_protos: u32,
    pub out_octets: u64,
    pub out_ucast_pkts: u32,
    pub out_multicast_pkts: u32,
    pub out_broadcast_pkts: u32,
    pub out_discards: u32,
    pub out_errors: u32,
    pub promiscuous_mode: u32,
}

#[derive(Debug)]
pub struct EthernetCounters {
    pub dot3_stats_alignment_errors: u32,
    pub dot3_stats_fcs_errors: u32,
    pub dot3_stats_single_collision_frames: u32,
    pub dot3_stats_multiple_collision_frames: u32,
    pub dot3_stats_sqe_test_errors: u32,
    pub dot3_stats_deferred_transmissions: u32,
    pub dot3_stats_late_collisions: u32,
    pub dot3_stats_excessive_collisions: u32,
    pub dot3_stats_internal_mac_transmit_errors: u32,
    pub dot3_stats_carrier_sense_errors: u32,
    pub dot3_stats_frame_too_longs: u32,
    pub dot3_stats_internal_mac_receive_errors: u32,
    pub dot3_stats_symbol_errors: u32,
}

#[derive(Debug)]
pub struct TokenRingCounters {
    pub dot5_stats_line_errors: u32,
    pub dot5_stats_burst_errors: u32,
    pub dot5_stats_ac_errors: u32,
    pub dot5_stats_abort_trans_errors: u32,
    pub dot5_stats_internal_errors: u32,
    pub dot5_stats_lost_frame_errors: u32,
    pub dot5_stats_receive_congestions: u32,
    pub dot5_stats_frame_copied_errors: u32,
    pub dot5_stats_token_errors: u32,
    pub dot5_stats_soft_errors: u32,
    pub dot5_stats_hard_errors: u32,
    pub dot5_stats_signal_loss: u32,
    pub dot5_stats_transmit_beacons: u32,
    pub dot5_stats_recoverys: u32,
    pub dot5_stats_lobe_wires: u32,
    pub dot5_stats_removes: u32,
    pub dot5_stats_singles: u32,
    pub dot5_stats_freq_errors: u32,
}

#[derive(Debug)]
pub struct VgCounters {
    pub dot12_in_high_priority_frames: u32,
    pub dot12_in_high_priority_octets: u64,
    pub dot12_in_norm_priority_frames: u32,
    pub dot12_in_norm_priority_octets: u64,
    pub dot12_in_ipm_errors: u32,
    pub dot12_in_oversize_frame_errors: u32,
    pub dot12_in_data_errors: u32,
    pub dot12_in_null_addressed_frames: u32,
    pub dot12_out_high_priority_frames: u32,
    pub dot12_out_high_priority_octets: u64,
    pub dot12_transition_into_trainings: u32,
    pub dot12_hc_in_high_priority_octets: u64,
    pub dot12_hc_in_norm_priority_octets: u64,
    pub dot12_hc_out_high_priority_octets: u64,
}

#[derive(Debug)]
pub struct Vlan {
    pub vlan_id: u32,
    pub octets: u64,
    pub ucast_pkts: u32,
    pub multicast_pkts: u32,
    pub broadcast_pkts: u32,
    pub discards: u32,
}

#[derive(Debug)]
pub struct Lane {
    pub lane_index: u32,      /* index of lane in module - starting from 1 */
    pub tx_bias_current: u32, /* microamps */
    pub tx_power: u32,        /* microwatts */
    pub tx_power_min: u32,    /* microwatts */
    pub tx_power_max: u32,    /* microwatts */
    pub tx_wavelength: u32,   /* nanometers */
    pub rx_power: u32,        /* microwatts */
    pub rx_power_min: u32,    /* microwatts */
    pub rx_power_max: u32,    /* microwatts */
    pub rx_wavelength: u32,   /* nanometers */
}

#[derive(Debug)]
pub struct Sfp {
    pub id: u32,
    pub total_lanes: u32,    /* total lanes in module */
    pub supply_voltage: u32, /* millivolts */
    pub temperature: i32,    /* signed - in oC / 1000 */
    pub lanes: Vec<Lane>,
}

#[derive(Debug)]
pub struct Processor {
    pub five_sec_cpu: u32, /* 5 second average CPU utilization */
    pub one_min_cpu: u32,  /* 1 minute average CPU utilization */
    pub five_min_cpu: u32, /* 5 minute average CPU utilization */
    pub total_memory: u64, /* total memory (in bytes) */
    pub free_memory: u64,  /* free memory (in bytes) */
}

#[derive(Debug)]
pub struct PortName {
    pub name: String,
}

#[derive(Debug)]
pub struct HostAdapter {
    pub if_index: u32,
    pub mac_addresses: Vec<[u8; 6]>,
}

/// Phyical or virtual network adapter NIC/vNIC
#[derive(Debug)]
pub struct HostAdapters {
    pub length: u32,
    pub adapters: Vec<HostAdapter>,
}

#[derive(Debug)]
pub struct HostParent {
    pub container_type: u32,
    pub container_index: u32,
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
    pub packets_in: u32,  /* total packets in */
    pub errs_in: u32,     /* total errors in */
    pub drops_in: u32,    /* total drops in */
    pub bytes_out: u64,   /* total bytes out */
    pub packets_out: u32, /* total packets out */
    pub errs_out: u32,    /* total errors out */
    pub drops_out: u32,   /* total drops out */
}

#[derive(Debug)]
pub struct Mib2IpGroup {
    pub forwarding: u32,
    pub default_ttl: u32,
    pub in_receives: u32,
    pub in_hdr_errors: u32,
    pub in_addr_errors: u32,
    pub forw_datagrams: u32,
    pub in_unknown_protos: u32,
    pub in_discards: u32,
    pub in_delivers: u32,
    pub out_requests: u32,
    pub out_discards: u32,
    pub out_no_routes: u32,
    pub reasm_timeout: u32,
    pub reasm_reqds: u32,
    pub reasm_oks: u32,
    pub reasm_fails: u32,
    pub frag_oks: u32,
    pub frag_fails: u32,
    pub frag_creates: u32,
}

#[derive(Debug)]
pub struct Mib2IcmpGroup {
    pub in_msgs: u32,
    pub in_errors: u32,
    pub in_dest_unreachs: u32,
    pub in_time_excds: u32,
    pub in_param_probs: u32,
    pub in_src_quenchs: u32,
    pub in_redirects: u32,
    pub in_echos: u32,
    pub in_echo_reps: u32,
    pub in_timestamps: u32,
    pub in_addr_masks: u32,
    pub in_addr_mask_reps: u32,
    pub out_msgs: u32,
    pub out_errors: u32,
    pub out_dest_unreachs: u32,
    pub out_time_excds: u32,
    pub out_param_probs: u32,
    pub out_src_quenchs: u32,
    pub out_redirects: u32,
    pub out_echos: u32,
    pub out_echo_reps: u32,
    pub out_timestamps: u32,
    pub out_timestamp_reps: u32,
    pub out_addr_masks: u32,
    pub out_addr_mask_reps: u32,
}

#[derive(Debug)]
pub struct Mib2TcpGroup {
    pub rto_algorithm: u32,
    pub rto_min: u32,
    pub rto_max: u32,
    pub max_conn: u32,
    pub active_opens: u32,
    pub passive_opens: u32,
    pub attempt_fails: u32,
    pub estab_resets: u32,
    pub curr_estab: u32,
    pub in_segs: u32,
    pub out_segs: u32,
    pub retrans_segs: u32,
    pub in_errs: u32,
    pub out_rsts: u32,
    pub in_csum_errs: u32,
}

#[derive(Debug)]
pub struct Mib2UdpGroup {
    pub in_datagrams: u32,
    pub no_ports: u32,
    pub in_errors: u32,
    pub out_datagrams: u32,
    pub rcvbuf_errors: u32,
    pub sndbuf_errors: u32,
    pub in_csum_errors: u32,
}

#[derive(Debug)]
pub struct VirtNode {
    pub mhz: u32,         /* expected CPU frequency */
    pub cpus: u32,        /* the number of active CPUs */
    pub memory: u64,      /* memory size in bytes */
    pub memory_free: u64, /* unassigned memory in bytes */
    pub num_domains: u32, /* number of active domains */
}

#[derive(Debug)]
pub struct VirtCpu {
    pub state: u32,       /* virtDomainState */
    pub cpu_time: u32,    /* the CPU time used (ms) */
    pub nr_virt_cpu: u32, /* number of virtual CPUs for the domain */
}

#[derive(Debug)]
pub struct VirtMemory {
    pub memory: u64,     /* memory in bytes used by domain */
    pub max_memory: u64, /* memory in bytes allowed */
}

#[derive(Debug)]
pub struct VirtDiskIO {
    pub capacity: u64,   /* logical size in bytes */
    pub allocation: u64, /* current allocation in bytes */
    pub available: u64,  /* remaining free bytes */
    pub rd_req: u32,     /* number of read requests */
    pub rd_bytes: u64,   /* number of read bytes */
    pub wr_req: u32,     /* number of write requests */
    pub wr_bytes: u64,   /* number of  written bytes */
    pub errs: u32,       /* read/write errors */
}

#[derive(Debug)]
pub struct VirtNetIO {
    pub rx_bytes: u64,   /* total bytes received */
    pub rx_packets: u32, /* total packets received */
    pub rx_errs: u32,    /* total receive errors */
    pub rx_drop: u32,    /* total receive drops */
    pub tx_bytes: u64,   /* total bytes transmitted */
    pub tx_packets: u32, /* total packets transmitted */
    pub tx_errs: u32,    /* total transmit errors */
    pub tx_drop: u32,    /* total transmit drops */
}

/// https://sflow.org/sflow_nvml.txt
#[derive(Debug)]
pub struct NvidiaGpu {
    pub device_count: u32, /* see nvmlDeviceGetCount */
    pub processes: u32,    /* see nvmlDeviceGetComputeRunningProcesses */
    pub gpu_time: u32,     /* total milliseconds in which one or more
                           kernels was executing on GPU
                           sum across all devices */
    pub mem_time: u32, /* total milliseconds during which global device
                       memory was being read/written
                       sum across all devices */
    pub mem_total: u64, /* sum of framebuffer memory across devices
                        see nvmlDeviceGetMemoryInfo */
    pub mem_free: u64, /* sum of free framebuffer memory across devices
                       see nvmlDeviceGetMemoryInfo */
    pub ecc_errors: u32, /* sum of volatile ECC errors across devices
                         see nvmlDeviceGetTotalEccErrors */
    pub energy: u32, /* sum of millijoules across devices
                     see nvmlDeviceGetPowerUsage */
    pub temperature: u32, /* maximum temperature in degrees Celsius
                          across devices
                          see nvmlDeviceGetTemperature */
    pub fan_speed: u32, /* maximum fan speed in percent across devices
                        see nvmlDeviceGetFanSpeed */
}

// Broadcom switch ASIC table utilizations
// opaque = counter_data; enterprise = 4413 (Broadcom); format = 3
//
// https://sflow.org/sflow_broadcom_tables.txt
#[derive(Debug)]
pub struct BcmTables {
    pub host_entries: u32,
    pub host_entries_max: u32,
    pub ipv4_entries: u32,
    pub ipv4_entries_max: u32,
    pub ipv6_entries: u32,
    pub ipv6_entries_max: u32,
    pub ipv4_ipv6_entries: u32,
    pub ipv4_ipv6_entries_max: u32,
    pub long_ipv6_entries: u32,
    pub long_ipv6_entries_max: u32,
    pub total_routes: u32,
    pub total_routes_max: u32,
    pub ecmp_nexthops: u32,
    pub ecmp_nexthops_max: u32,
    pub mac_entries: u32,
    pub mac_entries_max: u32,
    pub ipv4_neighbors: u32,
    pub ipv6_neighbors: u32,
    pub ipv4_routes: u32,
    pub ipv6_routes: u32,
    pub acl_ingress_entries: u32,
    pub acl_ingress_entries_max: u32,
    pub acl_ingress_counters: u32,
    pub acl_ingress_counters_max: u32,
    pub acl_ingress_meters: u32,
    pub acl_ingress_meters_max: u32,
    pub acl_ingress_slices: u32,
    pub acl_ingress_slices_max: u32,
    pub acl_egress_entries: u32,
    pub acl_egress_entries_max: u32,
    pub acl_egress_counters: u32,
    pub acl_egress_counters_max: u32,
    pub acl_egress_meters: u32,
    pub acl_egress_meters_max: u32,
    pub acl_egress_slices: u32,
    pub acl_egress_slices_max: u32,
}

#[derive(Debug)]
pub enum CounterRecordData {
    Interface(IfCounters),
    Ethernet(EthernetCounters),
    TokenRing(TokenRingCounters),
    VgCounters(VgCounters),
    Vlan(Vlan),
    Sfp(Sfp),
    Processor(Processor),
    PortName(PortName),
    HostDescription(HostDescription),
    HostAdapters(HostAdapters),
    HostParent(HostParent),
    HostCPU(HostCPU),
    HostMemory(HostMemory),
    HostDiskIO(HostDiskIO),
    HostNetIO(HostNetIO),
    Mib2IpGroup(Mib2IpGroup),
    Mib2IcmpGroup(Mib2IcmpGroup),
    Mib2TcpGroup(Mib2TcpGroup),
    Mib2UdpGroup(Mib2UdpGroup),
    VirtNode(VirtNode),
    VirtCpu(VirtCpu),
    VirtMemory(VirtMemory),
    VirtDisk(VirtDiskIO),
    VirtNetIO(VirtNetIO),
    NvidiaGpu(NvidiaGpu),
    BcmTables(BcmTables),

    Raw(u32, Vec<u8>),
}

#[derive(Debug)]
pub struct CounterRecord {
    pub header: RecordHeader,
    pub data: CounterRecordData,
}

#[derive(Debug)]
pub struct Sample {
    pub format: u32,
    pub length: u32,

    pub sample_sequence_number: u32,
    pub source_id_type: u32,
    pub source_id_value: u32,

    pub data: SampleData,
}

#[derive(Debug)]
pub enum SampleData {
    Flow {
        sampling_rate: u32,
        sample_pool: u32,
        drops: u32,
        input: u32,
        output: u32,
        records: Vec<FlowRecord>,
    },
    Counter {
        records: Vec<CounterRecord>,
    },
    ExpandedFlow {
        sampling_rate: u32,
        sample_pool: u32,
        drops: u32,
        input_if_format: u32,
        input_if_value: u32,
        output_if_format: u32,
        output_if_value: u32,
        records: Vec<FlowRecord>,
    },
    Drop {
        drops: u32,
        input: u32,
        output: u32,
        reason: u32,
        records: Vec<FlowRecord>,
    },
}

#[derive(Debug)]
pub struct Datagram {
    pub version: u32,
    pub agent_ip: IpAddr,
    pub sub_agent_id: u32,
    pub sequence_number: u32,
    pub uptime: u32,
    pub samples_count: u32,

    pub samples: Vec<Sample>,
}

fn decode_ipaddr(buf: &mut Cursor<&[u8]>) -> Result<IpAddr, Error> {
    let version = buf.try_get_u32()?;
    let ip_addr = if version == 1 {
        let mut octets = [0u8; 4];
        buf.read_exact(&mut octets)?;
        IpAddr::from(octets)
    } else if version == 2 {
        let mut octets = [0u8; 16];
        buf.read_exact(&mut octets)?;
        IpAddr::from(octets)
    } else {
        return Err(Error::UnknownIpVersion(version));
    };

    Ok(ip_addr)
}

#[inline]
fn decode_mac(buf: &mut Cursor<&[u8]>) -> Result<[u8; 6], Error> {
    let mut data = [0u8; 6];

    buf.read_exact(&mut data)?;
    buf.advance(2); // aligned to 4

    Ok(data)
}

// https://sflow.org/developers/structures.php
fn decode_counter_record(buf: &mut Cursor<&[u8]>) -> Result<CounterRecord, Error> {
    // read header first
    let data_format = buf.try_get_u32()?;
    let length = buf.try_get_u32()?;

    let data = match data_format {
        COUNTER_TYPE_INTERFACE => {
            let index = buf.try_get_u32()?;
            let typ = buf.try_get_u32()?;
            let speed = buf.try_get_u64()?;
            let direction = buf.try_get_u32()?;
            let status = buf.try_get_u32()?;
            let in_octets = buf.try_get_u64()?;
            let in_ucast_pkts = buf.try_get_u32()?;
            let in_multicast_pkts = buf.try_get_u32()?;
            let in_broadcast_pkts = buf.try_get_u32()?;
            let in_discards = buf.try_get_u32()?;
            let in_errors = buf.try_get_u32()?;
            let in_unknown_protos = buf.try_get_u32()?;
            let out_octets = buf.try_get_u64()?;
            let out_ucast_pkts = buf.try_get_u32()?;
            let out_multicast_pkts = buf.try_get_u32()?;
            let out_broadcast_pkts = buf.try_get_u32()?;
            let out_discards = buf.try_get_u32()?;
            let out_errors = buf.try_get_u32()?;
            let promiscuous_mode = buf.try_get_u32()?;

            CounterRecordData::Interface(IfCounters {
                index,
                typ,
                speed,
                direction,
                status,
                in_octets,
                in_ucast_pkts,
                in_multicast_pkts,
                in_broadcast_pkts,
                in_discards,
                in_errors,
                in_unknown_protos,
                out_octets,
                out_ucast_pkts,
                out_multicast_pkts,
                out_broadcast_pkts,
                out_discards,
                out_errors,
                promiscuous_mode,
            })
        }
        COUNTER_TYPE_ETHERNET => {
            let dot3_stats_alignment_errors = buf.try_get_u32()?;
            let dot3_stats_fcs_errors = buf.try_get_u32()?;
            let dot3_stats_single_collision_frames = buf.try_get_u32()?;
            let dot3_stats_multiple_collision_frames = buf.try_get_u32()?;
            let dot3_stats_sqe_test_errors = buf.try_get_u32()?;
            let dot3_stats_deferred_transmissions = buf.try_get_u32()?;
            let dot3_stats_late_collisions = buf.try_get_u32()?;
            let dot3_stats_excessive_collisions = buf.try_get_u32()?;
            let dot3_stats_internal_mac_transmit_errors = buf.try_get_u32()?;
            let dot3_stats_carrier_sense_errors = buf.try_get_u32()?;
            let dot3_stats_frame_too_longs = buf.try_get_u32()?;
            let dot3_stats_internal_mac_receive_errors = buf.try_get_u32()?;
            let dot3_stats_symbol_errors = buf.try_get_u32()?;

            CounterRecordData::Ethernet(EthernetCounters {
                dot3_stats_alignment_errors,
                dot3_stats_fcs_errors,
                dot3_stats_single_collision_frames,
                dot3_stats_multiple_collision_frames,
                dot3_stats_sqe_test_errors,
                dot3_stats_deferred_transmissions,
                dot3_stats_late_collisions,
                dot3_stats_excessive_collisions,
                dot3_stats_internal_mac_transmit_errors,
                dot3_stats_carrier_sense_errors,
                dot3_stats_frame_too_longs,
                dot3_stats_internal_mac_receive_errors,
                dot3_stats_symbol_errors,
            })
        }
        COUNTER_TYPE_TOKEN_RING => {
            let dot5_stats_line_errors = buf.try_get_u32()?;
            let dot5_stats_burst_errors = buf.try_get_u32()?;
            let dot5_stats_ac_errors = buf.try_get_u32()?;
            let dot5_stats_abort_trans_errors = buf.try_get_u32()?;
            let dot5_stats_internal_errors = buf.try_get_u32()?;
            let dot5_stats_lost_frame_errors = buf.try_get_u32()?;
            let dot5_stats_receive_congestions = buf.try_get_u32()?;
            let dot5_stats_frame_copied_errors = buf.try_get_u32()?;
            let dot5_stats_token_errors = buf.try_get_u32()?;
            let dot5_stats_soft_errors = buf.try_get_u32()?;
            let dot5_stats_hard_errors = buf.try_get_u32()?;
            let dot5_stats_signal_loss = buf.try_get_u32()?;
            let dot5_stats_transmit_beacons = buf.try_get_u32()?;
            let dot5_stats_recoverys = buf.try_get_u32()?;
            let dot5_stats_lobe_wires = buf.try_get_u32()?;
            let dot5_stats_removes = buf.try_get_u32()?;
            let dot5_stats_singles = buf.try_get_u32()?;
            let dot5_stats_freq_errors = buf.try_get_u32()?;

            CounterRecordData::TokenRing(TokenRingCounters {
                dot5_stats_line_errors,
                dot5_stats_burst_errors,
                dot5_stats_ac_errors,
                dot5_stats_abort_trans_errors,
                dot5_stats_internal_errors,
                dot5_stats_lost_frame_errors,
                dot5_stats_receive_congestions,
                dot5_stats_frame_copied_errors,
                dot5_stats_token_errors,
                dot5_stats_soft_errors,
                dot5_stats_hard_errors,
                dot5_stats_signal_loss,
                dot5_stats_transmit_beacons,
                dot5_stats_recoverys,
                dot5_stats_lobe_wires,
                dot5_stats_removes,
                dot5_stats_singles,
                dot5_stats_freq_errors,
            })
        }
        COUNTER_TYPE_VG => {
            let dot12_in_high_priority_frames = buf.try_get_u32()?;
            let dot12_in_high_priority_octets = buf.try_get_u64()?;
            let dot12_in_norm_priority_frames = buf.try_get_u32()?;
            let dot12_in_norm_priority_octets = buf.try_get_u64()?;
            let dot12_in_ipm_errors = buf.try_get_u32()?;
            let dot12_in_oversize_frame_errors = buf.try_get_u32()?;
            let dot12_in_data_errors = buf.try_get_u32()?;
            let dot12_in_null_addressed_frames = buf.try_get_u32()?;
            let dot12_out_high_priority_frames = buf.try_get_u32()?;
            let dot12_out_high_priority_octets = buf.try_get_u64()?;
            let dot12_transition_into_trainings = buf.try_get_u32()?;
            let dot12_hc_in_high_priority_octets = buf.try_get_u64()?;
            let dot12_hc_in_norm_priority_octets = buf.try_get_u64()?;
            let dot12_hc_out_high_priority_octets = buf.try_get_u64()?;

            CounterRecordData::VgCounters(VgCounters {
                dot12_in_high_priority_frames,
                dot12_in_high_priority_octets,
                dot12_in_norm_priority_frames,
                dot12_in_norm_priority_octets,
                dot12_in_ipm_errors,
                dot12_in_oversize_frame_errors,
                dot12_in_data_errors,
                dot12_in_null_addressed_frames,
                dot12_out_high_priority_frames,
                dot12_out_high_priority_octets,
                dot12_transition_into_trainings,
                dot12_hc_in_high_priority_octets,
                dot12_hc_in_norm_priority_octets,
                dot12_hc_out_high_priority_octets,
            })
        }
        COUNTER_TYPE_VLAN => {
            let vlan_id = buf.try_get_u32()?;
            let octets = buf.try_get_u64()?;
            let ucast_pkts = buf.try_get_u32()?;
            let multicast_pkts = buf.try_get_u32()?;
            let broadcast_pkts = buf.try_get_u32()?;
            let discards = buf.try_get_u32()?;

            CounterRecordData::Vlan(Vlan {
                vlan_id,
                octets,
                ucast_pkts,
                multicast_pkts,
                broadcast_pkts,
                discards,
            })
        }
        COUNTER_TYPE_SFP => {
            let id = buf.try_get_u32()?;
            let total_lanes = buf.try_get_u32()?;
            let supply_voltage = buf.try_get_u32()?;
            let temperature = buf.try_get_i32()?;
            let length = buf.try_get_u32()?;
            let mut lanes = Vec::with_capacity(length as usize);
            for _ in 0..length {
                let lane_index = buf.try_get_u32()?;
                let tx_bias_current = buf.try_get_u32()?;
                let tx_power = buf.try_get_u32()?;
                let tx_power_min = buf.try_get_u32()?;
                let tx_power_max = buf.try_get_u32()?;
                let tx_wavelength = buf.try_get_u32()?;
                let rx_power = buf.try_get_u32()?;
                let rx_power_min = buf.try_get_u32()?;
                let rx_power_max = buf.try_get_u32()?;
                let rx_wavelength = buf.try_get_u32()?;

                lanes.push(Lane {
                    lane_index,
                    tx_bias_current,
                    tx_power,
                    tx_power_min,
                    tx_power_max,
                    tx_wavelength,
                    rx_power,
                    rx_power_min,
                    rx_power_max,
                    rx_wavelength,
                })
            }

            CounterRecordData::Sfp(Sfp {
                id,
                total_lanes,
                supply_voltage,
                temperature,
                lanes,
            })
        }
        COUNTER_TYPE_HOST_CPU => {
            let load_one = buf.try_get_f32()?;
            let load_five = buf.try_get_f32()?;
            let load_fifteen = buf.try_get_f32()?;

            let proc_run = buf.try_get_u32()?;
            let proc_total = buf.try_get_u32()?;
            let cpu_num = buf.try_get_u32()?;
            let cpu_speed = buf.try_get_u32()?;
            let uptime = buf.try_get_u32()?;
            let cpu_user = buf.try_get_u32()?;
            let cpu_nice = buf.try_get_u32()?;
            let cpu_system = buf.try_get_u32()?;
            let cpu_idle = buf.try_get_u32()?;
            let cpu_wio = buf.try_get_u32()?;
            let cpu_intr = buf.try_get_u32()?;
            let cpu_sintr = buf.try_get_u32()?;
            let interrupts = buf.try_get_u32()?;
            let contexts = buf.try_get_u32()?;

            let cpu_steal = buf.try_get_u32()?;
            let cpu_guest = buf.try_get_u32()?;
            let cpu_guest_nice = buf.try_get_u32()?;

            CounterRecordData::HostCPU(HostCPU {
                load_one,
                load_five,
                load_fifteen,
                proc_run,
                proc_total,
                cpu_num,
                cpu_speed,
                uptime,
                cpu_user,
                cpu_nice,
                cpu_system,
                cpu_idle,
                cpu_wio,
                cpu_intr,
                cpu_sintr,
                interrupts,
                contexts,

                cpu_steal,
                cpu_guest,
                cpu_guest_nice,
            })
        }
        COUNTER_TYPE_PROCESSOR => {
            let five_sec_cpu = buf.try_get_u32()?;
            let one_min_cpu = buf.try_get_u32()?;
            let five_min_cpu = buf.try_get_u32()?;
            let total_memory = buf.try_get_u64()?;
            let free_memory = buf.try_get_u64()?;

            CounterRecordData::Processor(Processor {
                five_sec_cpu,
                one_min_cpu,
                five_min_cpu,
                total_memory,
                free_memory,
            })
        }
        COUNTER_TYPE_PORT_NAME => {
            let name = try_read_string(buf)?;
            CounterRecordData::PortName(PortName { name })
        }
        COUNTER_TYPE_HOST_DESCRIPTION => {
            let host = try_read_string(buf)?;

            let mut uuid = [0u8; 16];
            buf.read_exact(&mut uuid)?;

            let machine_type = buf.try_get_u32()?;
            let os_name = buf.try_get_u32()?;

            let os_release = try_read_string(buf)?;

            CounterRecordData::HostDescription(HostDescription {
                host,
                uuid,
                machine_type,
                os_name,
                os_release,
            })
        }
        COUNTER_TYPE_HOST_ADAPTERS => {
            let length = buf.try_get_u32()?;
            let mut adapters = Vec::with_capacity(length as usize);
            for _ in 0..length {
                let if_index = buf.try_get_u32()?;

                let count = buf.try_get_u32()?;
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
        COUNTER_TYPE_HOST_PARENT => {
            let container_type = buf.try_get_u32()?;
            let container_index = buf.try_get_u32()?;

            CounterRecordData::HostParent(HostParent {
                container_type,
                container_index,
            })
        }
        COUNTER_TYPE_HOST_MEMORY => {
            let mem_total = buf.try_get_u64()?;
            let mem_free = buf.try_get_u64()?;
            let mem_shared = buf.try_get_u64()?;
            let mem_buffers = buf.try_get_u64()?;
            let mem_cached = buf.try_get_u64()?;
            let swap_total = buf.try_get_u64()?;
            let swap_free = buf.try_get_u64()?;
            let page_in = buf.try_get_u32()?;
            let page_out = buf.try_get_u32()?;
            let swap_in = buf.try_get_u32()?;
            let swap_out = buf.try_get_u32()?;

            CounterRecordData::HostMemory(HostMemory {
                mem_total,
                mem_free,
                mem_shared,
                mem_buffers,
                mem_cached,
                swap_total,
                swap_free,
                page_in,
                page_out,
                swap_in,
                swap_out,
            })
        }
        COUNTER_TYPE_HOST_DISK_IO => {
            let disk_total = buf.try_get_u64()?;
            let disk_free = buf.try_get_u64()?;
            let part_max_used = buf.try_get_u32()?;
            let reads = buf.try_get_u32()?;
            let bytes_read = buf.try_get_u64()?;
            let read_time = buf.try_get_u32()?;
            let writes = buf.try_get_u32()?;
            let bytes_written = buf.try_get_u64()?;
            let write_time = buf.try_get_u32()?;

            CounterRecordData::HostDiskIO(HostDiskIO {
                disk_total,
                disk_free,
                part_max_used,
                reads,
                bytes_read,
                read_time,
                writes,
                bytes_written,
                write_time,
            })
        }
        COUNTER_TYPE_HOST_NET_IO => {
            let bytes_in = buf.try_get_u64()?;
            let packets_in = buf.try_get_u32()?;
            let errs_in = buf.try_get_u32()?;
            let drops_in = buf.try_get_u32()?;
            let bytes_out = buf.try_get_u64()?;
            let packets_out = buf.try_get_u32()?;
            let errs_out = buf.try_get_u32()?;
            let drops_out = buf.try_get_u32()?;

            CounterRecordData::HostNetIO(HostNetIO {
                bytes_in,
                packets_in,
                errs_in,
                drops_in,
                bytes_out,
                packets_out,
                errs_out,
                drops_out,
            })
        }
        COUNTER_TYPE_MIB2_IP_GROUP => {
            let forwarding = buf.try_get_u32()?;
            let default_ttl = buf.try_get_u32()?;
            let in_receives = buf.try_get_u32()?;
            let in_hdr_errors = buf.try_get_u32()?;
            let in_addr_errors = buf.try_get_u32()?;
            let forw_datagrams = buf.try_get_u32()?;
            let in_unknown_protos = buf.try_get_u32()?;
            let in_discards = buf.try_get_u32()?;
            let in_delivers = buf.try_get_u32()?;
            let out_requests = buf.try_get_u32()?;
            let out_discards = buf.try_get_u32()?;
            let out_no_routes = buf.try_get_u32()?;
            let reasm_timeout = buf.try_get_u32()?;
            let reasm_reqds = buf.try_get_u32()?;
            let reasm_oks = buf.try_get_u32()?;
            let reasm_fails = buf.try_get_u32()?;
            let frag_oks = buf.try_get_u32()?;
            let frag_fails = buf.try_get_u32()?;
            let frag_creates = buf.try_get_u32()?;

            CounterRecordData::Mib2IpGroup(Mib2IpGroup {
                forwarding,
                default_ttl,
                in_receives,
                in_hdr_errors,
                in_addr_errors,
                forw_datagrams,
                in_unknown_protos,
                in_discards,
                in_delivers,
                out_requests,
                out_discards,
                out_no_routes,
                reasm_timeout,
                reasm_reqds,
                reasm_oks,
                reasm_fails,
                frag_oks,
                frag_fails,
                frag_creates,
            })
        }
        COUNTER_TYPE_MIB2_ICMP_GROUP => {
            let in_msgs = buf.try_get_u32()?;
            let in_errors = buf.try_get_u32()?;
            let in_dest_unreachs = buf.try_get_u32()?;
            let in_time_excds = buf.try_get_u32()?;
            let in_param_probs = buf.try_get_u32()?;
            let in_src_quenchs = buf.try_get_u32()?;
            let in_redirects = buf.try_get_u32()?;
            let in_echos = buf.try_get_u32()?;
            let in_echo_reps = buf.try_get_u32()?;
            let in_timestamps = buf.try_get_u32()?;
            let in_addr_masks = buf.try_get_u32()?;
            let in_addr_mask_reps = buf.try_get_u32()?;
            let out_msgs = buf.try_get_u32()?;
            let out_errors = buf.try_get_u32()?;
            let out_dest_unreachs = buf.try_get_u32()?;
            let out_time_excds = buf.try_get_u32()?;
            let out_param_probs = buf.try_get_u32()?;
            let out_src_quenchs = buf.try_get_u32()?;
            let out_redirects = buf.try_get_u32()?;
            let out_echos = buf.try_get_u32()?;
            let out_echo_reps = buf.try_get_u32()?;
            let out_timestamps = buf.try_get_u32()?;
            let out_timestamp_reps = buf.try_get_u32()?;
            let out_addr_masks = buf.try_get_u32()?;
            let out_addr_mask_reps = buf.try_get_u32()?;

            CounterRecordData::Mib2IcmpGroup(Mib2IcmpGroup {
                in_msgs,
                in_errors,
                in_dest_unreachs,
                in_time_excds,
                in_param_probs,
                in_src_quenchs,
                in_redirects,
                in_echos,
                in_echo_reps,
                in_timestamps,
                in_addr_masks,
                in_addr_mask_reps,
                out_msgs,
                out_errors,
                out_dest_unreachs,
                out_time_excds,
                out_param_probs,
                out_src_quenchs,
                out_redirects,
                out_echos,
                out_echo_reps,
                out_timestamps,
                out_timestamp_reps,
                out_addr_masks,
                out_addr_mask_reps,
            })
        }
        COUNTER_TYPE_MIB2_TCP_GROUP => {
            let rto_algorithm = buf.try_get_u32()?;
            let rto_min = buf.try_get_u32()?;
            let rto_max = buf.try_get_u32()?;
            let max_conn = buf.try_get_u32()?;
            let active_opens = buf.try_get_u32()?;
            let passive_opens = buf.try_get_u32()?;
            let attempt_fails = buf.try_get_u32()?;
            let estab_resets = buf.try_get_u32()?;
            let curr_estab = buf.try_get_u32()?;
            let in_segs = buf.try_get_u32()?;
            let out_segs = buf.try_get_u32()?;
            let retrans_segs = buf.try_get_u32()?;
            let in_errs = buf.try_get_u32()?;
            let out_rsts = buf.try_get_u32()?;
            let in_csum_errs = buf.try_get_u32()?;

            CounterRecordData::Mib2TcpGroup(Mib2TcpGroup {
                rto_algorithm,
                rto_min,
                rto_max,
                max_conn,
                active_opens,
                passive_opens,
                attempt_fails,
                estab_resets,
                curr_estab,
                in_segs,
                out_segs,
                retrans_segs,
                in_errs,
                out_rsts,
                in_csum_errs,
            })
        }
        COUNTER_TYPE_MIB2_UDP_GROUP => {
            let in_datagrams = buf.try_get_u32()?;
            let no_ports = buf.try_get_u32()?;
            let in_errors = buf.try_get_u32()?;
            let out_datagrams = buf.try_get_u32()?;
            let rcvbuf_errors = buf.try_get_u32()?;
            let sndbuf_errors = buf.try_get_u32()?;
            let in_csum_errors = buf.try_get_u32()?;

            CounterRecordData::Mib2UdpGroup(Mib2UdpGroup {
                in_datagrams,
                no_ports,
                in_errors,
                out_datagrams,
                rcvbuf_errors,
                sndbuf_errors,
                in_csum_errors,
            })
        }
        COUNTER_TYPE_VIRT_NODE => {
            let mhz = buf.try_get_u32()?;
            let cpus = buf.try_get_u32()?;
            let memory = buf.try_get_u64()?;
            let memory_free = buf.try_get_u64()?;
            let num_domains = buf.try_get_u32()?;

            CounterRecordData::VirtNode(VirtNode {
                mhz,
                cpus,
                memory,
                memory_free,
                num_domains,
            })
        }
        COUNTER_TYPE_VIRT_CPU => {
            let state = buf.try_get_u32()?;
            let cpu_time = buf.try_get_u32()?;
            let nr_virt_cpu = buf.try_get_u32()?;

            CounterRecordData::VirtCpu(VirtCpu {
                state,
                cpu_time,
                nr_virt_cpu,
            })
        }
        COUNTER_TYPE_VIRT_MEMORY => {
            let memory = buf.try_get_u64()?;
            let max_memory = buf.try_get_u64()?;

            CounterRecordData::VirtMemory(VirtMemory { memory, max_memory })
        }
        COUNTER_TYPE_VIRT_DISK_IO => {
            let capacity = buf.try_get_u64()?;
            let allocation = buf.try_get_u64()?;
            let available = buf.try_get_u64()?;
            let rd_req = buf.try_get_u32()?;
            let rd_bytes = buf.try_get_u64()?;
            let wr_req = buf.try_get_u32()?;
            let wr_bytes = buf.try_get_u64()?;
            let errs = buf.try_get_u32()?;

            CounterRecordData::VirtDisk(VirtDiskIO {
                capacity,
                allocation,
                available,
                rd_req,
                rd_bytes,
                wr_req,
                wr_bytes,
                errs,
            })
        }

        COUNTER_TYPE_VIRT_NET_IO => {
            let rx_bytes = buf.try_get_u64()?;
            let rx_packets = buf.try_get_u32()?;
            let rx_errs = buf.try_get_u32()?;
            let rx_drop = buf.try_get_u32()?;
            let tx_bytes = buf.try_get_u64()?;
            let tx_packets = buf.try_get_u32()?;
            let tx_errs = buf.try_get_u32()?;
            let tx_drop = buf.try_get_u32()?;

            CounterRecordData::VirtNetIO(VirtNetIO {
                rx_bytes,
                rx_packets,
                rx_errs,
                rx_drop,
                tx_bytes,
                tx_packets,
                tx_errs,
                tx_drop,
            })
        }

        COUNTER_TYPE_NVIDIA_GPU => {
            let device_count = buf.try_get_u32()?;
            let processes = buf.try_get_u32()?;
            let gpu_time = buf.try_get_u32()?;
            let mem_time = buf.try_get_u32()?;
            let mem_total = buf.try_get_u64()?;
            let mem_free = buf.try_get_u64()?;
            let ecc_errors = buf.try_get_u32()?;
            let energy = buf.try_get_u32()?;
            let temperature = buf.try_get_u32()?;
            let fan_speed = buf.try_get_u32()?;

            CounterRecordData::NvidiaGpu(NvidiaGpu {
                device_count,
                processes,
                gpu_time,
                mem_time,
                mem_total,
                mem_free,
                ecc_errors,
                energy,
                temperature,
                fan_speed,
            })
        }

        COUNTER_TYPE_BCM_TABLES => {
            let host_entries = buf.try_get_u32()?;
            let host_entries_max = buf.try_get_u32()?;
            let ipv4_entries = buf.try_get_u32()?;
            let ipv4_entries_max = buf.try_get_u32()?;
            let ipv6_entries = buf.try_get_u32()?;
            let ipv6_entries_max = buf.try_get_u32()?;
            let ipv4_ipv6_entries = buf.try_get_u32()?;
            let ipv4_ipv6_entries_max = buf.try_get_u32()?;
            let long_ipv6_entries = buf.try_get_u32()?;
            let long_ipv6_entries_max = buf.try_get_u32()?;
            let total_routes = buf.try_get_u32()?;
            let total_routes_max = buf.try_get_u32()?;
            let ecmp_nexthops = buf.try_get_u32()?;
            let ecmp_nexthops_max = buf.try_get_u32()?;
            let mac_entries = buf.try_get_u32()?;
            let mac_entries_max = buf.try_get_u32()?;
            let ipv4_neighbors = buf.try_get_u32()?;
            let ipv6_neighbors = buf.try_get_u32()?;
            let ipv4_routes = buf.try_get_u32()?;
            let ipv6_routes = buf.try_get_u32()?;
            let acl_ingress_entries = buf.try_get_u32()?;
            let acl_ingress_entries_max = buf.try_get_u32()?;
            let acl_ingress_counters = buf.try_get_u32()?;
            let acl_ingress_counters_max = buf.try_get_u32()?;
            let acl_ingress_meters = buf.try_get_u32()?;
            let acl_ingress_meters_max = buf.try_get_u32()?;
            let acl_ingress_slices = buf.try_get_u32()?;
            let acl_ingress_slices_max = buf.try_get_u32()?;
            let acl_egress_entries = buf.try_get_u32()?;
            let acl_egress_entries_max = buf.try_get_u32()?;
            let acl_egress_counters = buf.try_get_u32()?;
            let acl_egress_counters_max = buf.try_get_u32()?;
            let acl_egress_meters = buf.try_get_u32()?;
            let acl_egress_meters_max = buf.try_get_u32()?;
            let acl_egress_slices = buf.try_get_u32()?;
            let acl_egress_slices_max = buf.try_get_u32()?;

            CounterRecordData::BcmTables(BcmTables {
                host_entries,
                host_entries_max,
                ipv4_entries,
                ipv4_entries_max,
                ipv6_entries,
                ipv6_entries_max,
                ipv4_ipv6_entries,
                ipv4_ipv6_entries_max,
                long_ipv6_entries,
                long_ipv6_entries_max,
                total_routes,
                total_routes_max,
                ecmp_nexthops,
                ecmp_nexthops_max,
                mac_entries,
                mac_entries_max,
                ipv4_neighbors,
                ipv6_neighbors,
                ipv4_routes,
                ipv6_routes,
                acl_ingress_entries,
                acl_ingress_entries_max,
                acl_ingress_counters,
                acl_ingress_counters_max,
                acl_ingress_meters,
                acl_ingress_meters_max,
                acl_ingress_slices,
                acl_ingress_slices_max,
                acl_egress_entries,
                acl_egress_entries_max,
                acl_egress_counters,
                acl_egress_counters_max,
                acl_egress_meters,
                acl_egress_meters_max,
                acl_egress_slices,
                acl_egress_slices_max,
            })
        }

        _ => {
            let mut data = vec![0u8; length as usize];
            buf.read_exact(&mut data)?;
            CounterRecordData::Raw(data_format, data)
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
    let data_format = buf.try_get_u32()?;
    let length = buf.try_get_u32()?;

    let record = match data_format {
        FLOW_TYPE_RAW => {
            let protocol = buf.try_get_u32()?;
            let frame_length = buf.try_get_u32()?;
            let stripped = buf.try_get_u32()?;
            let original_length = buf.try_get_u32()?;

            let mut header_bytes = vec![0; length as usize - 4 * 4];
            buf.read_exact(&mut header_bytes)?;

            FlowRecord::Raw(FlowRecordRaw {
                protocol,
                frame_length,
                stripped,
                original_length,
                header_bytes,
            })
        }
        FLOW_TYPE_EXT_LINUX_REASON => {
            let reason = try_read_string(buf)?;
            FlowRecord::ExtendedLinuxReason(ExtendedLinuxReason { reason })
        }
        FLOW_TYPE_ETH => {
            let length = buf.try_get_u32()?;
            let mut src_mac = [0u8; 6];
            buf.read_exact(&mut src_mac)?;
            let mut dst_mac = [0u8; 6];
            buf.read_exact(&mut dst_mac)?;
            let eth_type = buf.try_get_u32()?;

            FlowRecord::SampledEthernet(FlowRecordSampleEthernet {
                length,
                src_mac,
                dst_mac,
                eth_type,
            })
        }
        FLOW_TYPE_IPV4 => {
            let length = buf.try_get_u32()?;
            let protocol = buf.try_get_u32()?;
            let mut data = [0u8; 4];
            buf.read_exact(&mut data)?;
            let src_ip = Ipv4Addr::from(data);
            let mut data = [0u8; 4];
            buf.read_exact(&mut data)?;
            let dst_ip = Ipv4Addr::from(data);
            let src_port = buf.try_get_u32()?;
            let dst_port = buf.try_get_u32()?;
            let tcp_flags = buf.try_get_u32()?;
            let tos = buf.try_get_u32()?;

            FlowRecord::SampledIpv4(SampledIpv4 {
                length,
                protocol,
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                tcp_flags,
                tos,
            })
        }
        FLOW_TYPE_IPV6 => {
            let length = buf.try_get_u32()?;
            let protocol = buf.try_get_u32()?;
            let mut data = [0u8; 16];
            buf.read_exact(&mut data)?;
            let src_ip = Ipv6Addr::from(data);
            let mut data = [0u8; 16];
            buf.read_exact(&mut data)?;
            let dst_ip = Ipv6Addr::from(data);
            let src_port = buf.try_get_u32()?;
            let dst_port = buf.try_get_u32()?;
            let tcp_flags = buf.try_get_u32()?;
            let priority = buf.try_get_u32()?;

            FlowRecord::SampledIpv6(SampledIpv6 {
                length,
                protocol,
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                tcp_flags,
                priority,
            })
        }
        FLOW_TYPE_EXT_SWITCH => {
            let src_vlan = buf.try_get_u32()?;
            let src_priority = buf.try_get_u32()?;
            let dst_vlan = buf.try_get_u32()?;
            let dst_priority = buf.try_get_u32()?;

            FlowRecord::ExtendedSwitch(ExtendedSwitch {
                src_vlan,
                src_priority,
                dst_vlan,
                dst_priority,
            })
        }
        FLOW_TYPE_EXT_ROUTER => {
            let next_hop = decode_ipaddr(buf)?;
            let src_mask_len = buf.try_get_u32()?;
            let dst_mask_len = buf.try_get_u32()?;

            FlowRecord::ExtendedRouter(ExtendedRouter {
                next_hop,
                src_mask_len,
                dst_mask_len,
            })
        }
        FLOW_TYPE_EXT_GATEWAY => {
            let next_hop = decode_ipaddr(buf)?;
            let r#as = buf.try_get_u32()?;
            let src_as = buf.try_get_u32()?;
            let src_peer_as = buf.try_get_u32()?;
            let as_destinations = buf.try_get_u32()?;

            let (as_path_type, as_path_length, as_path) = if as_destinations != 0 {
                let as_path_type = buf.try_get_u32()?;
                let as_path_length = buf.try_get_u32()?;

                // protection for as-path length
                if as_path_length > 1000 {
                    return Err(Error::TooManyAsPath);
                }
                if as_path_length > buf.remaining() as u32 - 4 {
                    return Err(Error::InvalidAsPathLength);
                }
                let mut as_path: Vec<u32> = Vec::with_capacity(as_path_length as usize);
                for _ in 0..as_path_length {
                    as_path.push(buf.try_get_u32()?);
                }

                (as_path_type, as_path_length, as_path)
            } else {
                (0, 0, vec![])
            };

            let communities_length = buf.try_get_u32()?;
            // protection for communities length
            if communities_length > 1000 {
                return Err(Error::TooManyCommunities);
            }
            if communities_length > buf.remaining() as u32 - 4 {
                return Err(Error::InvalidCommunitiesLength);
            }
            let mut communities = Vec::with_capacity(communities_length as usize);
            for _ in 0..communities_length {
                communities.push(buf.try_get_u32()?);
            }

            let local_pref = buf.try_get_u32()?;

            FlowRecord::ExtendedGateway(ExtendedGateway {
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
            queue: buf.try_get_u32()?,
        }),
        FLOW_TYPE_EXT_ACL => {
            let number = buf.try_get_u32()?;
            let name = try_read_string(buf)?;
            let direction = buf.try_get_u32()?;

            FlowRecord::ExtendedACL(ExtendedACL {
                number,
                name,
                direction,
            })
        }
        FLOW_TYPE_EXT_FUNCTION => {
            let symbol = try_read_string(buf)?;

            FlowRecord::ExtendedFunction(ExtendedFunction { symbol })
        }
        FLOW_TYPE_EXT_TCP_INFO => {
            let direction = buf.try_get_u32()?;
            let snd_mss = buf.try_get_u32()?;
            let rcv_mss = buf.try_get_u32()?;
            let unacked = buf.try_get_u32()?;
            let lost = buf.try_get_u32()?;
            let retrans = buf.try_get_u32()?;
            let pmtu = buf.try_get_u32()?;
            let rtt = buf.try_get_u32()?;
            let rttvar = buf.try_get_u32()?;
            let snd_cwnd = buf.try_get_u32()?;
            let reordering = buf.try_get_u32()?;
            let min_rtt = buf.try_get_u32()?;

            FlowRecord::ExtendedTCPInfo(ExtendedTCPInfo {
                direction,
                snd_mss,
                rcv_mss,
                unacked,
                lost,
                retrans,
                pmtu,
                rtt,
                rttvar,
                snd_cwnd,
                reordering,
                min_rtt,
            })
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
    let format = buf.try_get_u32()?;
    let length = buf.try_get_u32()?;
    let sample_sequence_number = buf.try_get_u32()?;

    let (source_id_type, source_id_value) = match format {
        SAMPLE_FORMAT_FLOW | SAMPLE_FORMAT_COUNTER => {
            // Interlaced data-source format
            let source_id = buf.try_get_u32()?;

            (source_id >> 24, source_id & 0x00FF_FFFF)
        }
        SAMPLE_FORMAT_EXPANDED_FLOW | SAMPLE_FORMAT_EXPANDED_COUNTER | SAMPLE_FORMAT_DROP => {
            // Explicit data-source format
            (buf.try_get_u32()?, buf.try_get_u32()?)
        }
        _ => return Err(Error::UnknownSampleFormat(format)),
    };

    let data = match format {
        SAMPLE_FORMAT_FLOW => {
            let sampling_rate = buf.try_get_u32()?;
            let sample_pool = buf.try_get_u32()?;
            let drops = buf.try_get_u32()?;
            let input = buf.try_get_u32()?;
            let output = buf.try_get_u32()?;
            let flow_records_count = buf.try_get_u32()?;

            if flow_records_count > 1000 {
                // protection against ddos
                return Err(Error::TooManyFlowRecords);
            }
            let mut records = Vec::with_capacity(flow_records_count as usize);
            for _ in 0..flow_records_count {
                records.push(decode_flow_record(buf)?);
            }

            SampleData::Flow {
                sampling_rate,
                sample_pool,
                drops,
                input,
                output,
                records,
            }
        }
        SAMPLE_FORMAT_COUNTER | SAMPLE_FORMAT_EXPANDED_COUNTER => {
            let counter_records_count = buf.try_get_u32()?;
            if counter_records_count > 1000 {
                return Err(Error::TooManyFlowRecords);
            }

            let mut records = Vec::with_capacity(counter_records_count as usize);
            for _ in 0..counter_records_count {
                records.push(decode_counter_record(buf)?);
            }

            SampleData::Counter { records }
        }
        SAMPLE_FORMAT_EXPANDED_FLOW => {
            let sampling_rate = buf.try_get_u32()?;
            let sample_pool = buf.try_get_u32()?;
            let drops = buf.try_get_u32()?;
            let input_if_format = buf.try_get_u32()?;
            let input_if_value = buf.try_get_u32()?;
            let output_if_format = buf.try_get_u32()?;
            let output_if_value = buf.try_get_u32()?;

            let flow_records_count = buf.try_get_u32()?;
            if flow_records_count > 1000 {
                // protection against ddos
                return Err(Error::TooManyFlowRecords);
            }
            let mut records = Vec::with_capacity(flow_records_count as usize);
            for _ in 0..flow_records_count {
                records.push(decode_flow_record(buf)?);
            }

            SampleData::ExpandedFlow {
                sampling_rate,
                sample_pool,
                drops,
                input_if_format,
                input_if_value,
                output_if_format,
                output_if_value,
                records,
            }
        }
        SAMPLE_FORMAT_DROP => {
            let drops = buf.try_get_u32()?;
            let input = buf.try_get_u32()?;
            let output = buf.try_get_u32()?;
            let reason = buf.try_get_u32()?;

            let flow_records_count = buf.try_get_u32()?;
            if flow_records_count > 1000 {
                // protection against ddos
                return Err(Error::TooManyFlowRecords);
            }
            let mut records = Vec::with_capacity(flow_records_count as usize);
            for _ in 0..flow_records_count {
                records.push(decode_flow_record(buf)?);
            }

            SampleData::Drop {
                drops,
                input,
                output,
                reason,
                records,
            }
        }
        _ => {
            return Err(Error::UnknownSampleFormat(format));
        }
    };

    Ok(Sample {
        format,
        length,
        sample_sequence_number,
        source_id_type,
        source_id_value,
        data,
    })
}

fn try_read_string(buf: &mut Cursor<&[u8]>) -> Result<String, Error> {
    let len = buf.try_get_u32()?;
    let aligned_len = (len + 3) & (!3); // align to 4

    let mut data = vec![0u8; aligned_len as usize];
    buf.read_exact(&mut data)?;
    data.truncate(len as usize);

    Ok(unsafe { String::from_utf8_unchecked(data) })
}

impl Datagram {
    pub fn decode(data: impl AsRef<[u8]>) -> Result<Datagram, Error> {
        let mut buf = Cursor::new(data.as_ref());
        let version = buf.try_get_u32()?;
        if version != 5 {
            return Err(Error::IncompatibleVersion);
        }

        let agent_ip = decode_ipaddr(&mut buf)?;
        let sub_agent_id = buf.try_get_u32()?;
        let sequence_number = buf.try_get_u32()?;
        let uptime = buf.try_get_u32()?;

        let samples_count = buf.try_get_u32()?;
        if samples_count > 1000 {
            return Err(Error::TooManySamples);
        }

        let mut samples = Vec::with_capacity(samples_count as usize);
        for _ in 0..samples_count {
            samples.push(decode_sample(&mut buf)?);
        }

        Ok(Datagram {
            version,
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

        match &datagram.samples.first().unwrap().data {
            SampleData::Drop { records, .. } => {
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

        match &(datagram.samples.first().unwrap().data) {
            SampleData::Drop { records, .. } => {
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
            0x61, 0x72, 0x00, 0x00,
        ];

        let datagram = Datagram::decode(data).unwrap();
        assert_eq!(datagram.samples_count, 1);
        assert_eq!(datagram.samples.len(), 1);

        match &(datagram.samples.first().unwrap().data) {
            SampleData::Drop { records, .. } => {
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
}
