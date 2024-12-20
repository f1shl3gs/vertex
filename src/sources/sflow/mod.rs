mod datagram;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use chrono::Utc;
use configurable::configurable_component;
use datagram::{
    CounterRecord, CounterRecordData, Datagram, EgressQueue, EthernetCounters, ExtendedACL,
    ExtendedFunction, ExtendedGateway, ExtendedLinuxReason, ExtendedRouter, ExtendedSwitch,
    ExtendedTCPInfo, FlowRecord, FlowRecordRaw, FlowRecordSampleEthernet, HostAdapters, HostCPU,
    HostDescription, HostDiskIO, HostMemory, HostNetIO, HostParent, IfCounters, Lane,
    Mib2IcmpGroup, Mib2IpGroup, Mib2TcpGroup, Mib2UdpGroup, PortName, Processor, Sample,
    SampleHeader, SampledIpv4, SampledIpv6, Sfp, VgCounters, Vlan,
};
use event::Event;
use framework::config::{Output, Resource, SourceConfig, SourceContext};
use framework::source::UdpSource;
use framework::{Error, Source};
use value::Value;

fn default_listen() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 6343)
}

#[configurable_component(source, name = "sflow")]
struct Config {
    #[serde(default = "default_listen")]
    listen: SocketAddr,

    /// Configures the receive buffer size using the "SO_RCVBUF" option on the socket.
    #[serde(default, with = "humanize::bytes::serde_option")]
    receive_buffer_bytes: Option<usize>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "sflow")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let source = SFlowSource;

        source.run(self.listen, self.receive_buffer_bytes, cx)
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::udp(self.listen)]
    }
}

struct SFlowSource;

impl UdpSource for SFlowSource {
    fn build_events(&self, peer: SocketAddr, data: &[u8]) -> Result<Vec<Event>, Error> {
        let datagram = Datagram::decode(data)?;

        let mut value = Value::Object(Default::default());
        let Datagram {
            agent_ip,
            sub_agent_id,
            sequence_number,
            uptime,
            samples_count,
            samples,
            ..
        } = datagram;

        value.insert("peer", peer.to_string());
        value.insert("received_timestamp", Utc::now());

        value.insert("agent_ip", agent_ip.to_string());
        value.insert("sub_agent_id", sub_agent_id);
        value.insert("sequence_number", sequence_number);
        value.insert("uptime", uptime);

        let mut array = Vec::with_capacity(samples_count as usize);
        for sample in samples {
            array.push(convert_sample(sample));
        }
        value.insert("samples", array);

        Ok(vec![Event::Log(value.into())])
    }
}

fn convert_sample(sample: Sample) -> Value {
    let mut value = Value::Object(Default::default());

    match sample {
        Sample::Flow {
            header:
                SampleHeader {
                    sample_sequence_number,
                    source_id_type,
                    source_id_value,
                    ..
                },
            sampling_rate,
            sample_pool,
            drops,
            input,
            output,
            flow_records_count,
            records,
        } => {
            // header
            value.insert("sample_sequence_number", sample_sequence_number);
            value.insert("source_id_type", source_id_type);
            value.insert("source_id_value", source_id_value);

            value.insert("sampling_rate", sampling_rate);
            value.insert("sample_pool", sample_pool);
            value.insert("drops", drops);
            value.insert("input", input);
            value.insert("output", output);

            let mut array = Vec::with_capacity(flow_records_count as usize);
            for record in records {
                array.push(convert_flow_record(record));
            }
            value.insert("records", array);
        }
        Sample::Counter {
            header:
                SampleHeader {
                    sample_sequence_number,
                    source_id_type,
                    source_id_value,
                    ..
                },
            counter_records_count,
            records,
            ..
        } => {
            // header
            value.insert("sample_sequence_number", sample_sequence_number);
            value.insert("source_id_type", source_id_type);
            value.insert("source_id_value", source_id_value);

            let mut array = Vec::with_capacity(counter_records_count as usize);
            for record in records {
                array.push(convert_counter_record(record));
            }
            value.insert("records", array);
        }
        Sample::ExpandedFlow {
            header:
                SampleHeader {
                    sample_sequence_number,
                    source_id_type,
                    source_id_value,
                    ..
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
            ..
        } => {
            // header
            value.insert("sample_sequence_number", sample_sequence_number);
            value.insert("source_id_type", source_id_type);
            value.insert("source_id_value", source_id_value);

            value.insert("sampling_rate", sampling_rate);
            value.insert("sample_pool", sample_pool);
            value.insert("drops", drops);
            value.insert("input_if_format", input_if_format);
            value.insert("input_if_value", input_if_value);
            value.insert("output_if_format", output_if_format);
            value.insert("output_if_value", output_if_value);

            let mut array = Vec::with_capacity(flow_records_count as usize);
            for record in records {
                array.push(convert_flow_record(record));
            }
            value.insert("records", array);
        }
        Sample::Drop {
            header:
                SampleHeader {
                    sample_sequence_number,
                    source_id_type,
                    source_id_value,
                    ..
                },
            drops,
            input,
            output,
            reason,
            flow_records_count,
            records,
            ..
        } => {
            // header
            value.insert("sample_sequence_number", sample_sequence_number);
            value.insert("source_id_type", source_id_type);
            value.insert("source_id_value", source_id_value);

            value.insert("drops", drops);
            value.insert("input", input);
            value.insert("output", output);
            value.insert("reason", reason);

            let mut array = Vec::with_capacity(flow_records_count as usize);
            for record in records {
                array.push(convert_flow_record(record));
            }
            value.insert("records", array);
        }
    }

    value
}

#[inline]
fn mac_to_string(mac: [u8; 6]) -> String {
    format!(
        "{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

fn convert_flow_record(record: FlowRecord) -> Value {
    let mut value = Value::Object(Default::default());

    match record {
        FlowRecord::Raw(FlowRecordRaw {
            protocol,
            frame_length,
            stripped,
            original_length,
            header_bytes,
        }) => {
            value.insert("protocol", protocol);
            value.insert("frame_length", frame_length);
            value.insert("stripped", stripped);
            value.insert("original_length", original_length);
            value.insert("header_bytes", header_bytes);
        }
        FlowRecord::ExtendedLinuxReason(ExtendedLinuxReason { reason }) => {
            value.insert("reason", reason);
        }
        FlowRecord::SampledEthernet(FlowRecordSampleEthernet {
            length,
            src_mac,
            dst_mac,
            eth_type,
        }) => {
            value.insert("length", length);
            value.insert("src_mac", mac_to_string(src_mac));
            value.insert("dst_mac", mac_to_string(dst_mac));
            value.insert("eth_type", eth_type);
        }
        FlowRecord::SampledIpv4(SampledIpv4 {
            protocol,
            src_ip,
            dst_ip,
            src_port,
            dst_port,
            tcp_flags,
            tos,
            ..
        }) => {
            value.insert("protocol", protocol);
            value.insert("src_ip", src_ip.to_string());
            value.insert("dst_ip", dst_ip.to_string());
            value.insert("src_port", src_port);
            value.insert("dst_port", dst_port);
            value.insert("tcp_flags", tcp_flags);
            value.insert("tos", tos);
        }
        FlowRecord::SampledIpv6(SampledIpv6 {
            protocol,
            src_ip,
            dst_ip,
            src_port,
            dst_port,
            tcp_flags,
            priority,
            ..
        }) => {
            value.insert("protocol", protocol);
            value.insert("src_ip", src_ip.to_string());
            value.insert("dst_ip", dst_ip.to_string());
            value.insert("src_port", src_port);
            value.insert("dst_port", dst_port);
            value.insert("tcp_flags", tcp_flags);
            value.insert("priority", priority);
        }
        FlowRecord::ExtendedSwitch(ExtendedSwitch {
            src_vlan,
            src_priority,
            dst_vlan,
            dst_priority,
        }) => {
            value.insert("src_vlan", src_vlan);
            value.insert("src_priority", src_priority);
            value.insert("dst_vlan", dst_vlan);
            value.insert("dst_priority", dst_priority);
        }
        FlowRecord::ExtendedRouter(ExtendedRouter {
            next_hop,
            src_mask_len,
            dst_mask_len,
            ..
        }) => {
            value.insert("next_hop", next_hop.to_string());
            value.insert("src_mask_len", src_mask_len);
            value.insert("dst_mask_len", dst_mask_len);
        }
        FlowRecord::ExtendedGateway(ExtendedGateway {
            next_hop,
            r#as,
            src_as,
            src_peer_as,
            as_destinations,
            as_path_type,
            as_path,
            communities,
            local_pref,
            ..
        }) => {
            value.insert("next_hop", next_hop.to_string());
            value.insert("as", r#as);
            value.insert("src_as", src_as);
            value.insert("src_peer_as", src_peer_as);
            value.insert("as_destinations", as_destinations);
            value.insert("as_path_type", as_path_type);
            value.insert("as_path", as_path);
            value.insert("communities", communities);
            value.insert("local_pref", local_pref);
        }
        FlowRecord::EgressQueue(EgressQueue { queue }) => {
            value.insert("queue", queue);
        }
        FlowRecord::ExtendedACL(ExtendedACL {
            number,
            name,
            direction,
        }) => {
            value.insert("number", number);
            value.insert("name", name);
            value.insert("direction", direction);
        }
        FlowRecord::ExtendedFunction(ExtendedFunction { symbol }) => {
            value.insert("symbol", symbol);
        }
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
        }) => {
            value.insert("direction", direction);
            value.insert("snd_mss", snd_mss);
            value.insert("rcv_mss", rcv_mss);
            value.insert("unacked", unacked);
            value.insert("lost", lost);
            value.insert("retrans", retrans);
            value.insert("pmtu", pmtu);
            value.insert("rtt", rtt);
            value.insert("rttvar", rttvar);
            value.insert("snd_cwnd", snd_cwnd);
            value.insert("reordering", reordering);
            value.insert("min_rtt", min_rtt);
        }
    }

    value
}

fn convert_counter_record(record: CounterRecord) -> Value {
    let mut value = Value::Object(Default::default());

    match record.data {
        CounterRecordData::Interface(IfCounters {
            if_index,
            if_type,
            if_speed,
            if_direction,
            if_status,
            if_in_octets,
            if_in_ucast_pkts,
            if_in_multicast_pkts,
            if_in_broadcast_pkts,
            if_in_discards,
            if_in_errors,
            if_in_unknown_protos,
            if_out_octets,
            if_out_ucast_pkts,
            if_out_multicast_pkts,
            if_out_broadcast_pkts,
            if_out_discards,
            if_out_errors,
            if_promiscuous_mode,
        }) => {
            value.insert("if_index", if_index);
            value.insert("if_type", if_type);
            value.insert("if_speed", if_speed);
            value.insert("if_direction", if_direction);
            value.insert("if_status", if_status);
            value.insert("if_in_octets", if_in_octets);
            value.insert("if_in_ucast_pkts", if_in_ucast_pkts);
            value.insert("if_in_multicast_pkts", if_in_multicast_pkts);
            value.insert("if_in_broadcast_pkts", if_in_broadcast_pkts);
            value.insert("if_in_discards", if_in_discards);
            value.insert("if_in_errors", if_in_errors);
            value.insert("if_in_unknown_protos", if_in_unknown_protos);
            value.insert("if_out_octets", if_out_octets);
            value.insert("if_out_ucast_pkts", if_out_ucast_pkts);
            value.insert("if_out_multicast_pkts", if_out_multicast_pkts);
            value.insert("if_out_broadcast_pkts", if_out_broadcast_pkts);
            value.insert("if_out_discards", if_out_discards);
            value.insert("if_out_errors", if_out_errors);
            value.insert("if_promiscuous_mode", if_promiscuous_mode);
        }
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
        }) => {
            value.insert("dot3_stats_alignment_errors", dot3_stats_alignment_errors);
            value.insert("dot3_stats_fcs_errors", dot3_stats_fcs_errors);
            value.insert(
                "dot3_stats_single_collision_frames",
                dot3_stats_single_collision_frames,
            );
            value.insert(
                "dot3_stats_multiple_collision_frames",
                dot3_stats_multiple_collision_frames,
            );
            value.insert("dot3_stats_sqe_test_errors", dot3_stats_sqe_test_errors);
            value.insert(
                "dot3_stats_deferred_transmissions",
                dot3_stats_deferred_transmissions,
            );
            value.insert("dot3_stats_late_collisions", dot3_stats_late_collisions);
            value.insert(
                "dot3_stats_excessive_collisions",
                dot3_stats_excessive_collisions,
            );
            value.insert(
                "dot3_stats_internal_mac_transmit_errors",
                dot3_stats_internal_mac_transmit_errors,
            );
            value.insert(
                "dot3_stats_carrier_sense_errors",
                dot3_stats_carrier_sense_errors,
            );
            value.insert("dot3_stats_frame_too_longs", dot3_stats_frame_too_longs);
            value.insert(
                "dot3_stats_internal_mac_receive_errors",
                dot3_stats_internal_mac_receive_errors,
            );
            value.insert("dot3_stats_symbol_errors", dot3_stats_symbol_errors);
        }
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
        }) => {
            value.insert(
                "dot12_in_high_priority_frames",
                dot12_in_high_priority_frames,
            );
            value.insert(
                "dot12_in_high_priority_octets",
                dot12_in_high_priority_octets,
            );
            value.insert(
                "dot12_in_norm_priority_frames",
                dot12_in_norm_priority_frames,
            );
            value.insert(
                "dot12_in_norm_priority_octets",
                dot12_in_norm_priority_octets,
            );
            value.insert("dot12_in_ipm_errors", dot12_in_ipm_errors);
            value.insert(
                "dot12_in_oversize_frame_errors",
                dot12_in_oversize_frame_errors,
            );
            value.insert("dot12_in_data_errors", dot12_in_data_errors);
            value.insert(
                "dot12_in_null_addressed_frames",
                dot12_in_null_addressed_frames,
            );
            value.insert(
                "dot12_out_high_priority_frames",
                dot12_out_high_priority_frames,
            );
            value.insert(
                "dot12_out_high_priority_octets",
                dot12_out_high_priority_octets,
            );
            value.insert(
                "dot12_transition_into_trainings",
                dot12_transition_into_trainings,
            );
            value.insert(
                "dot12_hc_in_high_priority_octets",
                dot12_hc_in_high_priority_octets,
            );
            value.insert(
                "dot12_hc_in_norm_priority_octets",
                dot12_hc_in_norm_priority_octets,
            );
            value.insert(
                "dot12_hc_out_high_priority_octets",
                dot12_hc_out_high_priority_octets,
            );
        }
        CounterRecordData::Vlan(Vlan {
            vlan_id,
            octets,
            ucast_pkts,
            multicast_pkts,
            broadcast_pkts,
            discards,
        }) => {
            value.insert("vlan_id", vlan_id);
            value.insert("octets", octets);
            value.insert("ucast_pkts", ucast_pkts);
            value.insert("multicast_pkts", multicast_pkts);
            value.insert("broadcast_pkts", broadcast_pkts);
            value.insert("discards", discards);
        }
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
        }) => {
            value.insert("load_one", load_one);
            value.insert("load_five", load_five);
            value.insert("load_fifteen", load_fifteen);
            value.insert("proc_run", proc_run);
            value.insert("proc_total", proc_total);
            value.insert("cpu_num", cpu_num);
            value.insert("cpu_speed", cpu_speed);
            value.insert("uptime", uptime);
            value.insert("cpu_user", cpu_user);
            value.insert("cpu_nice", cpu_nice);
            value.insert("cpu_system", cpu_system);
            value.insert("cpu_idle", cpu_idle);
            value.insert("cpu_wio", cpu_wio);
            value.insert("cpu_intr", cpu_intr);
            value.insert("cpu_sintr", cpu_sintr);
            value.insert("interrupts", interrupts);
            value.insert("contexts", contexts);

            value.insert("cpu_steal", cpu_steal);
            value.insert("cpu_guest", cpu_guest);
            value.insert("cpu_guest_nice", cpu_guest_nice);
        }
        CounterRecordData::Processor(Processor {
            five_sec_cpu,
            one_min_cpu,
            five_min_cpu,
            total_memory,
            free_memory,
        }) => {
            value.insert("five_sec_cpu", five_sec_cpu);
            value.insert("one_min_cpu", one_min_cpu);
            value.insert("five_min_cpu", five_min_cpu);
            value.insert("total_memory", total_memory);
            value.insert("free_memory", free_memory);
        }
        CounterRecordData::HostAdapters(HostAdapters { length, adapters }) => {
            let mut array = Vec::with_capacity(length as usize);
            for adapter in adapters {
                let mut item = Value::Object(Default::default());
                item.insert("if_index", adapter.if_index);

                let mut mac_addresses = Vec::with_capacity(adapter.mac_addresses.len());
                for mac in adapter.mac_addresses {
                    mac_addresses.push(Value::from(mac_to_string(mac)));
                }
                item.insert("mac_addresses", mac_addresses);

                array.push(item);
            }

            value.insert("host_adapters", array);
        }
        CounterRecordData::HostDescription(HostDescription {
            host,
            uuid,
            machine_type,
            os_name,
            os_release,
        }) => {
            value.insert("host", host);
            value.insert("uuid", format_uuid(uuid));
            value.insert("machine_type", machine_type_to_string(machine_type));
            value.insert("os_name", os_name_to_string(os_name));
            value.insert("os_release", os_release);
        }
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
        }) => {
            value.insert("mem_total", mem_total);
            value.insert("mem_free", mem_free);
            value.insert("mem_shared", mem_shared);
            value.insert("mem_buffers", mem_buffers);
            value.insert("mem_cached", mem_cached);
            value.insert("swap_total", swap_total);
            value.insert("swap_free", swap_free);
            value.insert("page_in", page_in);
            value.insert("page_out", page_out);
            value.insert("swap_in", swap_in);
            value.insert("swap_out", swap_out);
        }
        CounterRecordData::HostNetIO(HostNetIO {
            bytes_in,
            packets_in,
            errs_in,
            drops_in,
            bytes_out,
            packets_out,
            errs_out,
            drops_out,
        }) => {
            value.insert("bytes_in", bytes_in);
            value.insert("packets_in", packets_in);
            value.insert("errs_in", errs_in);
            value.insert("drops_in", drops_in);
            value.insert("bytes_out", bytes_out);
            value.insert("packets_out", packets_out);
            value.insert("errs_out", errs_out);
            value.insert("drops_out", drops_out);
        }
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
        }) => {
            value.insert("disk_total", disk_total);
            value.insert("disk_free", disk_free);
            value.insert("part_max_used", part_max_used);
            value.insert("reads", reads);
            value.insert("bytes_read", bytes_read);
            value.insert("read_time", read_time);
            value.insert("writes", writes);
            value.insert("bytes_written", bytes_written);
            value.insert("write_time", write_time);
        }
        CounterRecordData::Mib2IpGroup(Mib2IpGroup {
            ip_forwarding,
            ip_default_ttl,
            ip_in_receives,
            ip_in_hdr_errors,
            ip_in_addr_errors,
            ip_forw_datagrams,
            ip_in_unknown_protos,
            ip_in_discards,
            ip_in_delivers,
            ip_out_requests,
            ip_out_discards,
            ip_out_no_routes,
            ip_reasm_timeout,
            ip_reasm_reqds,
            ip_reasm_oks,
            ip_reasm_fails,
            ip_frag_oks,
            ip_frag_fails,
            ip_frag_creates,
        }) => {
            value.insert("ip_forwarding", ip_forwarding);
            value.insert("ip_default_ttl", ip_default_ttl);
            value.insert("ip_in_receives", ip_in_receives);
            value.insert("ip_in_hdr_errors", ip_in_hdr_errors);
            value.insert("ip_in_addr_errors", ip_in_addr_errors);
            value.insert("ip_forw_datagrams", ip_forw_datagrams);
            value.insert("ip_in_unknown_protos", ip_in_unknown_protos);
            value.insert("ip_in_discards", ip_in_discards);
            value.insert("ip_in_delivers", ip_in_delivers);
            value.insert("ip_out_requests", ip_out_requests);
            value.insert("ip_out_discards", ip_out_discards);
            value.insert("ip_out_no_routes", ip_out_no_routes);
            value.insert("ip_reasm_timeout", ip_reasm_timeout);
            value.insert("ip_reasm_reqds", ip_reasm_reqds);
            value.insert("ip_reasm_oks", ip_reasm_oks);
            value.insert("ip_reasm_fails", ip_reasm_fails);
            value.insert("ip_frag_oks", ip_frag_oks);
            value.insert("ip_frag_fails", ip_frag_fails);
            value.insert("ip_frag_creates", ip_frag_creates);
        }
        CounterRecordData::Mib2IcmpGroup(Mib2IcmpGroup {
            icmp_in_msgs,
            icmp_in_errors,
            icmp_in_dest_unreachs,
            icmp_in_time_excds,
            icmp_in_param_probs,
            icmp_in_src_quenchs,
            icmp_in_redirects,
            icmp_in_echos,
            icmp_in_echo_reps,
            icmp_in_timestamps,
            icmp_in_addr_masks,
            icmp_in_addr_mask_reps,
            icmp_out_msgs,
            icmp_out_errors,
            icmp_out_dest_unreachs,
            icmp_out_time_excds,
            icmp_out_param_probs,
            icmp_out_src_quenchs,
            icmp_out_redirects,
            icmp_out_echos,
            icmp_out_echo_reps,
            icmp_out_timestamps,
            icmp_out_timestamp_reps,
            icmp_out_addr_masks,
            icmp_out_addr_mask_reps,
        }) => {
            value.insert("icmp_in_msgs", icmp_in_msgs);
            value.insert("icmp_in_errors", icmp_in_errors);
            value.insert("icmp_in_dest_unreachs", icmp_in_dest_unreachs);
            value.insert("icmp_in_time_excds", icmp_in_time_excds);
            value.insert("icmp_in_param_probs", icmp_in_param_probs);
            value.insert("icmp_in_src_quenchs", icmp_in_src_quenchs);
            value.insert("icmp_in_redirects", icmp_in_redirects);
            value.insert("icmp_in_echos", icmp_in_echos);
            value.insert("icmp_in_echo_reps", icmp_in_echo_reps);
            value.insert("icmp_in_timestamps", icmp_in_timestamps);
            value.insert("icmp_in_addr_masks", icmp_in_addr_masks);
            value.insert("icmp_in_addr_mask_reps", icmp_in_addr_mask_reps);
            value.insert("icmp_out_msgs", icmp_out_msgs);
            value.insert("icmp_out_errors", icmp_out_errors);
            value.insert("icmp_out_dest_unreachs", icmp_out_dest_unreachs);
            value.insert("icmp_out_time_excds", icmp_out_time_excds);
            value.insert("icmp_out_param_probs", icmp_out_param_probs);
            value.insert("icmp_out_src_quenchs", icmp_out_src_quenchs);
            value.insert("icmp_out_redirects", icmp_out_redirects);
            value.insert("icmp_out_echos", icmp_out_echos);
            value.insert("icmp_out_echo_reps", icmp_out_echo_reps);
            value.insert("icmp_out_timestamps", icmp_out_timestamps);
            value.insert("icmp_out_timestamp_reps", icmp_out_timestamp_reps);
            value.insert("icmp_out_addr_masks", icmp_out_addr_masks);
            value.insert("icmp_out_addr_mask_reps", icmp_out_addr_mask_reps);
        }
        CounterRecordData::Mib2TcpGroup(Mib2TcpGroup {
            tcp_rto_algorithm,
            tcp_rto_min,
            tcp_rto_max,
            tcp_max_conn,
            tcp_active_opens,
            tcp_passive_opens,
            tcp_attempt_fails,
            tcp_estab_resets,
            tcp_curr_estab,
            tcp_in_segs,
            tcp_out_segs,
            tcp_retrans_segs,
            tcp_in_errs,
            tcp_out_rsts,
            tcp_in_csum_errs,
        }) => {
            value.insert("tcp_rto_algorithm", tcp_rto_algorithm);
            value.insert("tcp_rto_min", tcp_rto_min);
            value.insert("tcp_rto_max", tcp_rto_max);
            value.insert("tcp_max_conn", tcp_max_conn);
            value.insert("tcp_active_opens", tcp_active_opens);
            value.insert("tcp_passive_opens", tcp_passive_opens);
            value.insert("tcp_attempt_fails", tcp_attempt_fails);
            value.insert("tcp_estab_resets", tcp_estab_resets);
            value.insert("tcp_curr_estab", tcp_curr_estab);
            value.insert("tcp_in_segs", tcp_in_segs);
            value.insert("tcp_out_segs", tcp_out_segs);
            value.insert("tcp_retrans_segs", tcp_retrans_segs);
            value.insert("tcp_in_errs", tcp_in_errs);
            value.insert("tcp_out_rsts", tcp_out_rsts);
            value.insert("tcp_in_csum_errs", tcp_in_csum_errs);
        }
        CounterRecordData::Mib2UdpGroup(Mib2UdpGroup {
            udp_in_datagrams,
            udp_no_ports,
            udp_in_errors,
            udp_out_datagrams,
            udp_rcvbuf_errors,
            udp_sndbuf_errors,
            udp_in_csum_errors,
        }) => {
            value.insert("udp_in_datagrams", udp_in_datagrams);
            value.insert("udp_no_ports", udp_no_ports);
            value.insert("udp_in_errors", udp_in_errors);
            value.insert("udp_out_datagrams", udp_out_datagrams);
            value.insert("udp_rcvbuf_errors", udp_rcvbuf_errors);
            value.insert("udp_sndbuf_errors", udp_sndbuf_errors);
            value.insert("udp_in_csum_errors", udp_in_csum_errors);
        }
        CounterRecordData::PortName(PortName { name }) => {
            value.insert("port_name", name);
        }
        CounterRecordData::HostParent(HostParent {
            container_type,
            container_index,
        }) => {
            value.insert("container_type", container_type);
            value.insert("container_index", container_index);
        }
        CounterRecordData::Sfp(Sfp {
            module_id,
            module_total_lanes,
            module_supply_voltage,
            module_temperature,
            lanes,
        }) => {
            value.insert("module_id", module_id);
            value.insert("module_total_lanes", module_total_lanes);
            value.insert("module_supply_voltage", module_supply_voltage);
            value.insert("module_temperature", module_temperature);
            let mut array = Vec::with_capacity(lanes.len());
            for Lane {
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
            } in lanes
            {
                let mut item = Value::Object(Default::default());
                item.insert("lane_index", lane_index);
                item.insert("tx_bias_current", tx_bias_current);
                item.insert("tx_power", tx_power);
                item.insert("tx_power_min", tx_power_min);
                item.insert("tx_power_max", tx_power_max);
                item.insert("tx_wavelength", tx_wavelength);
                item.insert("rx_power", rx_power);
                item.insert("rx_power_min", rx_power_min);
                item.insert("rx_power_max", rx_power_max);
                item.insert("rx_wavelength", rx_wavelength);

                array.push(item);
            }
        }
        CounterRecordData::Raw(format, data) => {
            value.insert("format", format);
            value.insert("data", data);
        }
    }

    value
}

// https://sflow.org/sflow_host.txt
fn os_name_to_string(n: u32) -> String {
    match n {
        0 => "unknown",
        1 => "other",
        2 => "linux",
        3 => "windows",
        4 => "darwin",
        5 => "hpux",
        6 => "aix",
        7 => "dragonfly",
        8 => "freebsd",
        9 => "netbsd",
        10 => "openbsd",
        11 => "osf",
        12 => "solaris",
        _ => "",
    }
    .to_string()
}

fn machine_type_to_string(n: u32) -> String {
    match n {
        0 => "unknown",
        1 => "other",
        2 => "x86",
        3 => "x86_64",
        4 => "ia64",
        5 => "sparc",
        6 => "alpha",
        7 => "powerpc",
        8 => "m68k",
        9 => "mips",
        10 => "arm",
        11 => "hppa",
        12 => "s390",
        _ => "",
    }
    .to_string()
}

// for the binary data to sth. like this
// 936DA01F-9ABD-4D9D-80C7-02AF85C822A8
fn format_uuid(data: [u8; 16]) -> String {
    const LOWER: [u8; 16] = [
        b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e',
        b'f',
    ];

    let groups = [(0, 8), (9, 13), (14, 18), (19, 23), (24, 36)];
    let mut dst = vec![0; 36];

    let mut group_idx = 0;
    let mut i = 0;
    while group_idx < 5 {
        let (start, end) = groups[group_idx];
        let mut j = start;
        while j < end {
            let x = data[i];
            i += 1;

            dst[j] = LOWER[(x >> 4) as usize];
            dst[j + 1] = LOWER[(x & 0x0f) as usize];
            j += 2;
        }
        if group_idx < 4 {
            dst[end] = b'-';
        }
        group_idx += 1;
    }

    unsafe { String::from_utf8_unchecked(dst) }
}
