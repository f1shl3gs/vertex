mod datagram;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use chrono::Utc;
use configurable::configurable_component;
use datagram::{CounterRecord, CounterRecordData, Datagram, FlowRecord, Lane, Sample, SampleData};
use event::{LogRecord, Metric, tags};
use framework::config::{OutputType, Resource, SourceConfig, SourceContext};
use framework::{Error, Source};
use value::{Value, value};

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
        Ok(Box::pin(run(self.listen, self.receive_buffer_bytes, cx)))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![
            OutputType::log().with_port("logs"),
            OutputType::metric().with_port("metrics"),
        ]
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::udp(self.listen)]
    }
}

async fn run(
    addr: SocketAddr,
    receive_buffer_bytes: Option<usize>,
    cx: SourceContext,
) -> Result<(), ()> {
    let socket = match tokio::net::UdpSocket::bind(addr).await {
        Ok(s) => s,
        Err(err) => {
            warn!(
                message = "bind UDP socket failed",
                %addr,
                %err
            );

            return Err(());
        }
    };

    if let Some(bytes) = receive_buffer_bytes
        && let Err(err) = framework::udp::set_receive_buffer_size(&socket, bytes)
    {
        warn!(
            message = "set receive buffer size failed",
            %addr,
            %err,
        );
    }

    let mut buf = [0u8; u16::MAX as usize];
    let mut shutdown = cx.shutdown;
    let mut output = cx.output;

    loop {
        let (size, peer) = tokio::select! {
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok(t) => t,
                    Err(err) => {
                        warn!(
                            message = "recv datagram failed",
                            %err,
                        );

                        return Err(());
                    }
                }
            },
            _ = &mut shutdown => break,
        };

        match build_events(&buf[..size]) {
            Ok((logs, metrics)) => {
                if !logs.is_empty()
                    && let Err(_err) = output.send_named("logs", logs.into()).await
                {
                    return Err(());
                }

                if !metrics.is_empty()
                    && let Err(_err) = output.send_named("metrics", metrics.into()).await
                {
                    return Err(());
                }
            }
            Err(err) => {
                warn!(
                    message = "build events failed",
                    %err,
                    %peer,
                    internal_log_rate_limit = 30,
                );

                // println!("{:?}", &buf[..size]);
            }
        }
    }

    Ok(())
}

fn build_events(data: &[u8]) -> Result<(Vec<LogRecord>, Vec<Metric>), Error> {
    let Datagram {
        agent_ip,
        sub_agent_id,
        sequence_number,
        uptime,
        samples,
        ..
    } = Datagram::decode(data)?;

    let (mut logs, mut metrics) = convert_samples(samples);

    // assume samples only contains one type of samples
    if !logs.is_empty() {
        let agent_ip = &agent_ip.to_string();
        logs.iter_mut().for_each(|log| {
            let metadata = log.metadata_mut().value_mut();

            metadata.insert(
                "sflow",
                value!({
                    "agent": agent_ip,
                    "sequence_number": sequence_number,
                    "sub_agent_id": sub_agent_id,
                    "uptime": uptime,
                }),
            );
        })
    }

    if !metrics.is_empty() {
        let timestamp = Utc::now();
        metrics.iter_mut().for_each(|m| {
            m.timestamp = Some(timestamp);
            m.insert_tag("agent", agent_ip.to_string());
        });
    }

    Ok((logs, metrics))
}

fn convert_samples(samples: Vec<Sample>) -> (Vec<LogRecord>, Vec<Metric>) {
    let mut metrics = vec![];
    let mut logs = vec![];

    for Sample {
        sample_sequence_number,
        source_id_type,
        source_id_value,
        data,
        ..
    } in samples
    {
        match data {
            SampleData::Flow {
                sampling_rate,
                sample_pool,
                drops,
                input,
                output,
                records,
            } => {
                let mut value = Value::Object(Default::default());

                // header
                value.insert("sample_sequence_number", sample_sequence_number);
                value.insert("source_id_type", source_id_type);
                value.insert("source_id_value", source_id_value);

                value.insert("sampling_rate", sampling_rate);
                value.insert("sample_pool", sample_pool);
                value.insert("drops", drops);
                value.insert("input", input);
                value.insert("output", output);

                let mut array = Vec::with_capacity(records.len());
                for record in records {
                    array.push(convert_flow_record(record));
                }
                value.insert("records", array);

                logs.push(LogRecord::from(value));
            }
            SampleData::Counter { records } => {
                // header
                let mut partial = vec![];
                for record in records {
                    partial.extend(convert_counter_record(record));
                }

                partial.iter_mut().for_each(|m| {
                    m.insert_tag("source_id", source_id_value);
                });

                metrics.extend(partial);
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
                ..
            } => {
                let mut value = Value::Object(Default::default());

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

                let mut array = Vec::with_capacity(records.len());
                for record in records {
                    array.push(convert_flow_record(record));
                }
                value.insert("records", array);

                logs.push(LogRecord::from(value));
            }
            SampleData::Drop {
                drops,
                input,
                output,
                reason,
                records,
                ..
            } => {
                let mut value = Value::Object(Default::default());

                // header
                value.insert("sample_sequence_number", sample_sequence_number);
                value.insert("source_id_type", source_id_type);
                value.insert("source_id_value", source_id_value);

                value.insert("drops", drops);
                value.insert("input", input);
                value.insert("output", output);
                value.insert("reason", reason);

                let mut array = Vec::with_capacity(records.len());
                for record in records {
                    array.push(convert_flow_record(record));
                }
                value.insert("records", array);

                logs.push(LogRecord::from(value));
            }
        }
    }

    (logs, metrics)
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
        FlowRecord::Raw {
            protocol,
            frame_length,
            stripped,
            original_length,
            header_bytes,
        } => {
            value.insert("protocol", protocol);
            value.insert("frame_length", frame_length);
            value.insert("stripped", stripped);
            value.insert("original_length", original_length);
            value.insert("header_bytes", header_bytes);
        }
        FlowRecord::ExtendedLinuxReason { reason } => {
            value.insert("reason", reason);
        }
        FlowRecord::SampledEthernet {
            length,
            src_mac,
            dst_mac,
            eth_type,
        } => {
            value.insert("length", length);
            value.insert("src_mac", mac_to_string(src_mac));
            value.insert("dst_mac", mac_to_string(dst_mac));
            value.insert("eth_type", eth_type);
        }
        FlowRecord::SampledIpv4 {
            protocol,
            src_ip,
            dst_ip,
            src_port,
            dst_port,
            tcp_flags,
            tos,
            ..
        } => {
            value.insert("protocol", protocol);
            value.insert("src_ip", src_ip.to_string());
            value.insert("dst_ip", dst_ip.to_string());
            value.insert("src_port", src_port);
            value.insert("dst_port", dst_port);
            value.insert("tcp_flags", tcp_flags);
            value.insert("tos", tos);
        }
        FlowRecord::SampledIpv6 {
            protocol,
            src_ip,
            dst_ip,
            src_port,
            dst_port,
            tcp_flags,
            priority,
            ..
        } => {
            value.insert("protocol", protocol);
            value.insert("src_ip", src_ip.to_string());
            value.insert("dst_ip", dst_ip.to_string());
            value.insert("src_port", src_port);
            value.insert("dst_port", dst_port);
            value.insert("tcp_flags", tcp_flags);
            value.insert("priority", priority);
        }
        FlowRecord::ExtendedSwitch {
            src_vlan,
            src_priority,
            dst_vlan,
            dst_priority,
        } => {
            value.insert("src_vlan", src_vlan);
            value.insert("src_priority", src_priority);
            value.insert("dst_vlan", dst_vlan);
            value.insert("dst_priority", dst_priority);
        }
        FlowRecord::ExtendedRouter {
            next_hop,
            src_mask_len,
            dst_mask_len,
            ..
        } => {
            value.insert("next_hop", next_hop.to_string());
            value.insert("src_mask_len", src_mask_len);
            value.insert("dst_mask_len", dst_mask_len);
        }
        FlowRecord::ExtendedGateway {
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
        } => {
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
        FlowRecord::EgressQueue { queue } => {
            value.insert("queue", queue);
        }
        FlowRecord::ExtendedACL {
            number,
            name,
            direction,
        } => {
            value.insert("number", number);
            value.insert("name", name);
            value.insert("direction", direction);
        }
        FlowRecord::ExtendedFunction { symbol } => {
            value.insert("symbol", symbol);
        }
        FlowRecord::ExtendedTCPInfo {
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
        } => {
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

fn convert_counter_record(record: CounterRecord) -> Vec<Metric> {
    match record.data {
        CounterRecordData::Interface {
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
        } => {
            let tags = tags!(
                "index" => index,
                "type" => typ,
            );

            vec![
                Metric::gauge_with_tags(
                    "sflow_interface_speed",
                    "the speed of this interface",
                    speed,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "sflow_interface_direction",
                    "derived from MAU MIB(RFC 2668) 0 = unknown, 1=full-duplex, 2=half-duplex, 3=in, 4=out",
                    direction,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "sflow_interface_admin_status",
                    "admin status of this interface, 0 = down, 1 = up",
                    (status & 0x1) != 0,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "sflow_interface_oper_status",
                    "oper status of this interface, 0 = down, 1 = up",
                    (status & 0x2) != 0,
                    tags.clone(),
                ),
                Metric::sum_with_tags("sflow_interface_in_octets", "", in_octets, tags.clone()),
                Metric::sum_with_tags(
                    "sflow_interface_in_ucast_pkts",
                    "",
                    in_ucast_pkts,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "sflow_interface_in_multicast_pkts",
                    "",
                    in_multicast_pkts,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "sflow_interface_in_broadcast_pkts",
                    "",
                    in_broadcast_pkts,
                    tags.clone(),
                ),
                Metric::sum_with_tags("sflow_interface_in_discards", "", in_discards, tags.clone()),
                Metric::sum_with_tags("sflow_interface_in_errors", "", in_errors, tags.clone()),
                Metric::sum_with_tags(
                    "sflow_interface_in_unknown_protos",
                    "",
                    in_unknown_protos,
                    tags.clone(),
                ),
                Metric::sum_with_tags("sflow_interface_out_octets", "", out_octets, tags.clone()),
                Metric::sum_with_tags(
                    "sflow_interface_out_ucast_pkts",
                    "",
                    out_ucast_pkts,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "sflow_interface_out_multicast_pkts",
                    "",
                    out_multicast_pkts,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "sflow_interface_out_broadcast_pkts",
                    "",
                    out_broadcast_pkts,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "sflow_interface_out_discards",
                    "",
                    out_discards,
                    tags.clone(),
                ),
                Metric::sum_with_tags("sflow_interface_out_errors", "", out_errors, tags.clone()),
                Metric::sum_with_tags(
                    "sflow_interface_promiscuous_mode",
                    "",
                    promiscuous_mode,
                    tags,
                ),
            ]
        }
        CounterRecordData::Ethernet {
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
        } => {
            vec![
                Metric::sum(
                    "dot3_stats_alignment_errors",
                    "",
                    dot3_stats_alignment_errors,
                ),
                Metric::sum("dot3_stats_fcs_errors", "", dot3_stats_fcs_errors),
                Metric::sum(
                    "dot3_stats_single_collision_frames",
                    "",
                    dot3_stats_single_collision_frames,
                ),
                Metric::sum(
                    "dot3_stats_multiple_collision_frames",
                    "",
                    dot3_stats_multiple_collision_frames,
                ),
                Metric::sum("dot3_stats_sqe_test_errors", "", dot3_stats_sqe_test_errors),
                Metric::sum(
                    "dot3_stats_deferred_transmissions",
                    "",
                    dot3_stats_deferred_transmissions,
                ),
                Metric::sum("dot3_stats_late_collisions", "", dot3_stats_late_collisions),
                Metric::sum(
                    "dot3_stats_excessive_collisions",
                    "",
                    dot3_stats_excessive_collisions,
                ),
                Metric::sum(
                    "dot3_stats_internal_mac_transmit_errors",
                    "",
                    dot3_stats_internal_mac_transmit_errors,
                ),
                Metric::sum(
                    "dot3_stats_carrier_sense_errors",
                    "",
                    dot3_stats_carrier_sense_errors,
                ),
                Metric::sum("dot3_stats_frame_too_longs", "", dot3_stats_frame_too_longs),
                Metric::sum(
                    "dot3_stats_internal_mac_receive_errors",
                    "",
                    dot3_stats_internal_mac_receive_errors,
                ),
                Metric::sum("dot3_stats_symbol_errors", "", dot3_stats_symbol_errors),
            ]
        }
        CounterRecordData::TokenRing {
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
        } => {
            vec![
                Metric::sum(
                    "sflow_token_ring_dot5_stats_line_errors",
                    "",
                    dot5_stats_line_errors,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_burst_errors",
                    "",
                    dot5_stats_burst_errors,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_ac_errors",
                    "",
                    dot5_stats_ac_errors,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_abort_trans_errors",
                    "",
                    dot5_stats_abort_trans_errors,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_internal_errors",
                    "",
                    dot5_stats_internal_errors,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_lost_frame_errors",
                    "",
                    dot5_stats_lost_frame_errors,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_receive_congestions",
                    "",
                    dot5_stats_receive_congestions,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_frame_copied_errors",
                    "",
                    dot5_stats_frame_copied_errors,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_token_errors",
                    "",
                    dot5_stats_token_errors,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_soft_errors",
                    "",
                    dot5_stats_soft_errors,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_hard_errors",
                    "",
                    dot5_stats_hard_errors,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_signal_loss",
                    "",
                    dot5_stats_signal_loss,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_transmit_beacons",
                    "",
                    dot5_stats_transmit_beacons,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_recoverys",
                    "",
                    dot5_stats_recoverys,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_lobe_wires",
                    "",
                    dot5_stats_lobe_wires,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_removes",
                    "",
                    dot5_stats_removes,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_singles",
                    "",
                    dot5_stats_singles,
                ),
                Metric::sum(
                    "sflow_token_ring_dot5_stats_freq_errors",
                    "",
                    dot5_stats_freq_errors,
                ),
            ]
        }
        CounterRecordData::VgCounters {
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
        } => {
            vec![
                Metric::sum(
                    "dot12_in_high_priority_frames",
                    "",
                    dot12_in_high_priority_frames,
                ),
                Metric::sum(
                    "dot12_in_high_priority_octets",
                    "",
                    dot12_in_high_priority_octets,
                ),
                Metric::sum(
                    "dot12_in_norm_priority_frames",
                    "",
                    dot12_in_norm_priority_frames,
                ),
                Metric::sum(
                    "dot12_in_norm_priority_octets",
                    "",
                    dot12_in_norm_priority_octets,
                ),
                Metric::sum("dot12_in_ipm_errors", "", dot12_in_ipm_errors),
                Metric::sum(
                    "dot12_in_oversize_frame_errors",
                    "",
                    dot12_in_oversize_frame_errors,
                ),
                Metric::sum("dot12_in_data_errors", "", dot12_in_data_errors),
                Metric::sum(
                    "dot12_in_null_addressed_frames",
                    "",
                    dot12_in_null_addressed_frames,
                ),
                Metric::sum(
                    "dot12_out_high_priority_frames",
                    "",
                    dot12_out_high_priority_frames,
                ),
                Metric::sum(
                    "dot12_out_high_priority_octets",
                    "",
                    dot12_out_high_priority_octets,
                ),
                Metric::sum(
                    "dot12_transition_into_trainings",
                    "",
                    dot12_transition_into_trainings,
                ),
                Metric::sum(
                    "dot12_hc_in_high_priority_octets",
                    "",
                    dot12_hc_in_high_priority_octets,
                ),
                Metric::sum(
                    "dot12_hc_in_norm_priority_octets",
                    "",
                    dot12_hc_in_norm_priority_octets,
                ),
                Metric::sum(
                    "dot12_hc_out_high_priority_octets",
                    "",
                    dot12_hc_out_high_priority_octets,
                ),
            ]
        }
        CounterRecordData::Vlan {
            vlan_id,
            octets,
            ucast_pkts,
            multicast_pkts,
            broadcast_pkts,
            discards,
        } => {
            let tags = tags!(
                "id" => vlan_id,
            );

            vec![
                Metric::sum_with_tags("sflow_vlan_octets", "", octets, tags.clone()),
                Metric::sum_with_tags("sflow_vlan_ucast_pkts", "", ucast_pkts, tags.clone()),
                Metric::sum_with_tags(
                    "sflow_vlan_multicast_pkts",
                    "",
                    multicast_pkts,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "sflow_vlan_broadcast_pkts",
                    "",
                    broadcast_pkts,
                    tags.clone(),
                ),
                Metric::sum_with_tags("sflow_vlan_discards", "", discards, tags),
            ]
        }
        CounterRecordData::HostCPU {
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
            ..
        } => {
            vec![
                Metric::gauge(
                    "sflow_host_cpu_load_one",
                    "1 minute load avg, -1 = unknown",
                    load_one,
                ),
                Metric::gauge(
                    "sflow_host_cpu_load_five",
                    "5 minute load avg, -1 = unknown",
                    load_five,
                ),
                Metric::gauge(
                    "sflow_host_cpu_load_fifteen",
                    "15 minute load avg, -1 = unknown",
                    load_fifteen,
                ),
                Metric::gauge(
                    "sflow_host_cpu_proc_run",
                    "total number of running processes",
                    proc_run,
                ),
                Metric::gauge(
                    "sflow_host_cpu_proc_total",
                    "total number of processes",
                    proc_total,
                ),
                Metric::gauge("sflow_host_cpu_num", "number of CPUs", cpu_num),
                Metric::gauge("sflow_host_cpu_speed", "speed in MHz of CPU", cpu_speed),
                Metric::gauge("sflow_host_cpu_uptime", "seconds since last reboot", uptime),
                Metric::gauge("sflow_host_cpu_user", "user time (ms)", cpu_user),
                Metric::gauge("sflow_host_cpu_nice", "nice time (ms)", cpu_nice),
                Metric::gauge("sflow_host_cpu_system", "system time (ms)", cpu_system),
                Metric::gauge("sflow_host_cpu_idle", "idle time (ms)", cpu_idle),
                Metric::gauge(
                    "sflow_host_cpu_wio",
                    "time waiting for I/O to complete (ms)",
                    cpu_wio,
                ),
                Metric::gauge(
                    "sflow_host_cpu_intr",
                    "time servicing interrupts (ms)",
                    cpu_intr,
                ),
                Metric::gauge(
                    "sflow_host_cpu_sintr",
                    "time servicing soft interrupts (ms)",
                    cpu_sintr,
                ),
                Metric::sum("sflow_host_cpu_interrupts", "interrupt count", interrupts),
                Metric::sum("sflow_host_cpu_contexts", "context switch count", contexts),
            ]
        }
        CounterRecordData::Processor {
            five_sec_cpu,
            one_min_cpu,
            five_min_cpu,
            total_memory,
            free_memory,
        } => {
            vec![
                Metric::gauge(
                    "sflow_processor_five_sec_cpu",
                    "5 second average CPU utilization",
                    five_sec_cpu,
                ),
                Metric::gauge(
                    "sflow_processor_one_min_cpu",
                    "1 minute average CPU utilization",
                    one_min_cpu,
                ),
                Metric::gauge(
                    "sflow_processor_five_min_cpu",
                    "5 minute average CPU utilization",
                    five_min_cpu,
                ),
                Metric::gauge(
                    "sflow_processor_total_memory",
                    "total memory (in bytes)",
                    total_memory,
                ),
                Metric::gauge(
                    "sflow_processor_free_memory",
                    "free memory (in bytes)",
                    free_memory,
                ),
            ]
        }
        CounterRecordData::HostAdapters { adapters, .. } => {
            let mut metrics = Vec::with_capacity(adapters.len());

            for adapter in adapters {
                metrics.push(Metric::gauge_with_tags(
                    "sflow_host_adapter_mac_addresses",
                    "Physical or virtual network adapter NIC/vNIC",
                    1,
                    tags!(
                        "if_index" => adapter.if_index,
                        "mac" => adapter.mac_addresses.len()
                    ),
                ));
            }

            metrics
        }
        CounterRecordData::HostDescription {
            host,
            uuid,
            machine_type,
            os_name,
            os_release,
        } => {
            vec![Metric::gauge_with_tags(
                "sflow_host_info",
                "physical or virtual host description",
                1,
                tags!(
                    "host" => host,
                    "uuid" => format_uuid(uuid),
                    "machine_type" => machine_type_to_string(machine_type),
                    "os_name" => os_name_to_string(os_name),
                    "os_release" => os_release,
                ),
            )]
        }
        CounterRecordData::HostMemory {
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
        } => {
            vec![
                Metric::gauge("sflow_host_mem_total", "", mem_total),
                Metric::gauge("sflow_host_mem_free", "", mem_free),
                Metric::gauge("sflow_host_mem_shared", "", mem_shared),
                Metric::gauge("sflow_host_mem_buffers", "", mem_buffers),
                Metric::gauge("sflow_host_mem_cached", "", mem_cached),
                Metric::gauge("sflow_host_swap_total", "", swap_total),
                Metric::gauge("sflow_host_swap_free", "", swap_free),
                Metric::gauge("sflow_host_page_in", "", page_in),
                Metric::gauge("sflow_host_page_out", "", page_out),
                Metric::gauge("sflow_host_swap_in", "", swap_in),
                Metric::gauge("sflow_host_swap_out", "", swap_out),
            ]
        }
        CounterRecordData::HostNetIO {
            bytes_in,
            packets_in,
            errs_in,
            drops_in,
            bytes_out,
            packets_out,
            errs_out,
            drops_out,
        } => {
            vec![
                Metric::sum("sflow_host_network_bytes_in", "", bytes_in),
                Metric::sum("sflow_host_network_packets_in", "", packets_in),
                Metric::sum("sflow_host_network_errs_in", "", errs_in),
                Metric::sum("sflow_host_network_drops_in", "", drops_in),
                Metric::sum("sflow_host_network_bytes_out", "", bytes_out),
                Metric::sum("sflow_host_network_packets_out", "", packets_out),
                Metric::sum("sflow_host_network_errs_out", "", errs_out),
                Metric::sum("sflow_host_network_drops_out", "", drops_out),
            ]
        }
        CounterRecordData::HostDiskIO {
            disk_total,
            disk_free,
            part_max_used,
            reads,
            bytes_read,
            read_time,
            writes,
            bytes_written,
            write_time,
        } => {
            vec![
                Metric::sum(
                    "sflow_host_disk_total",
                    "total disk size in bytes",
                    disk_total,
                ),
                Metric::sum(
                    "sflow_host_disk_free",
                    "total disk free in bytes",
                    disk_free,
                ),
                Metric::sum(
                    "sflow_host_part_max_used",
                    "utilization of most utilized partition",
                    part_max_used,
                ),
                Metric::sum("sflow_host_reads", "reads issued", reads),
                Metric::sum("sflow_host_bytes_read", "bytes read", bytes_read),
                Metric::sum("sflow_host_read_time", "read time (ms)", read_time),
                Metric::sum("sflow_host_writes", "writes completed", writes),
                Metric::sum("sflow_host_bytes_written", "bytes written", bytes_written),
                Metric::sum("sflow_host_write_time", "write time (ms)", write_time),
            ]
        }
        CounterRecordData::Mib2IpGroup {
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
        } => {
            vec![
                Metric::sum("sflow_ip_forwarding", "", forwarding),
                Metric::sum("sflow_ip_default_ttl", "", default_ttl),
                Metric::sum("sflow_ip_in_receives", "", in_receives),
                Metric::sum("sflow_ip_in_hdr_errors", "", in_hdr_errors),
                Metric::sum("sflow_ip_in_addr_errors", "", in_addr_errors),
                Metric::sum("sflow_ip_forw_datagrams", "", forw_datagrams),
                Metric::sum("sflow_ip_in_unknown_protos", "", in_unknown_protos),
                Metric::sum("sflow_ip_in_discards", "", in_discards),
                Metric::sum("sflow_ip_in_delivers", "", in_delivers),
                Metric::sum("sflow_ip_out_requests", "", out_requests),
                Metric::sum("sflow_ip_out_discards", "", out_discards),
                Metric::sum("sflow_ip_out_no_routes", "", out_no_routes),
                Metric::sum("sflow_ip_reasm_timeout", "", reasm_timeout),
                Metric::sum("sflow_ip_reasm_reqds", "", reasm_reqds),
                Metric::sum("sflow_ip_reasm_oks", "", reasm_oks),
                Metric::sum("sflow_ip_reasm_fails", "", reasm_fails),
                Metric::sum("sflow_ip_frag_oks", "", frag_oks),
                Metric::sum("sflow_ip_frag_fails", "", frag_fails),
                Metric::sum("sflow_ip_frag_creates", "", frag_creates),
            ]
        }
        CounterRecordData::Mib2IcmpGroup {
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
        } => {
            vec![
                Metric::sum("sflow_icmp_in_msgs", "", in_msgs),
                Metric::sum("sflow_icmp_in_errors", "", in_errors),
                Metric::sum("sflow_icmp_in_dest_unreachs", "", in_dest_unreachs),
                Metric::sum("sflow_icmp_in_time_excds", "", in_time_excds),
                Metric::sum("sflow_icmp_in_param_probs", "", in_param_probs),
                Metric::sum("sflow_icmp_in_src_quenchs", "", in_src_quenchs),
                Metric::sum("sflow_icmp_in_redirects", "", in_redirects),
                Metric::sum("sflow_icmp_in_echos", "", in_echos),
                Metric::sum("sflow_icmp_in_echo_reps", "", in_echo_reps),
                Metric::sum("sflow_icmp_in_timestamps", "", in_timestamps),
                Metric::sum("sflow_icmp_in_addr_masks", "", in_addr_masks),
                Metric::sum("sflow_icmp_in_addr_mask_reps", "", in_addr_mask_reps),
                Metric::sum("sflow_icmp_out_msgs", "", out_msgs),
                Metric::sum("sflow_icmp_out_errors", "", out_errors),
                Metric::sum("sflow_icmp_out_dest_unreachs", "", out_dest_unreachs),
                Metric::sum("sflow_icmp_out_time_excds", "", out_time_excds),
                Metric::sum("sflow_icmp_out_param_probs", "", out_param_probs),
                Metric::sum("sflow_icmp_out_src_quenchs", "", out_src_quenchs),
                Metric::sum("sflow_icmp_out_redirects", "", out_redirects),
                Metric::sum("sflow_icmp_out_echos", "", out_echos),
                Metric::sum("sflow_icmp_out_echo_reps", "", out_echo_reps),
                Metric::sum("sflow_icmp_out_timestamps", "", out_timestamps),
                Metric::sum("sflow_icmp_out_timestamp_reps", "", out_timestamp_reps),
                Metric::sum("sflow_icmp_out_addr_masks", "", out_addr_masks),
                Metric::sum("sflow_icmp_out_addr_mask_reps", "", out_addr_mask_reps),
            ]
        }
        CounterRecordData::Mib2TcpGroup {
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
        } => {
            vec![
                Metric::sum("sflow_tcp_rto_algorithm", "", rto_algorithm),
                Metric::sum("sflow_tcp_rto_min", "", rto_min),
                Metric::sum("sflow_tcp_rto_max", "", rto_max),
                Metric::sum("sflow_tcp_max_conn", "", max_conn),
                Metric::sum("sflow_tcp_active_opens", "", active_opens),
                Metric::sum("sflow_tcp_passive_opens", "", passive_opens),
                Metric::sum("sflow_tcp_attempt_fails", "", attempt_fails),
                Metric::sum("sflow_tcp_estab_resets", "", estab_resets),
                Metric::sum("sflow_tcp_curr_estab", "", curr_estab),
                Metric::sum("sflow_tcp_in_segs", "", in_segs),
                Metric::sum("sflow_tcp_out_segs", "", out_segs),
                Metric::sum("sflow_tcp_retrans_segs", "", retrans_segs),
                Metric::sum("sflow_tcp_in_errs", "", in_errs),
                Metric::sum("sflow_tcp_out_rsts", "", out_rsts),
                Metric::sum("sflow_tcp_in_csum_errs", "", in_csum_errs),
            ]
        }
        CounterRecordData::Mib2UdpGroup {
            in_datagrams,
            no_ports,
            in_errors,
            out_datagrams,
            rcvbuf_errors,
            sndbuf_errors,
            in_csum_errors,
        } => {
            vec![
                Metric::sum("sflow_udp_in_datagrams", "", in_datagrams),
                Metric::sum("sflow_udp_no_ports", "", no_ports),
                Metric::sum("sflow_udp_in_errors", "", in_errors),
                Metric::sum("sflow_udp_out_datagrams", "", out_datagrams),
                Metric::sum("sflow_udp_rcvbuf_errors", "", rcvbuf_errors),
                Metric::sum("sflow_udp_sndbuf_errors", "", sndbuf_errors),
                Metric::sum("sflow_udp_in_csum_errors", "", in_csum_errors),
            ]
        }
        CounterRecordData::PortName { .. } => {
            vec![]
        }
        CounterRecordData::HostParent { .. } => {
            vec![]
        }
        CounterRecordData::Sfp {
            id,
            total_lanes,
            supply_voltage,
            temperature,
            lanes,
        } => {
            let mut metrics = Vec::with_capacity(2 + lanes.len() * 9);

            metrics.extend([
                Metric::gauge_with_tags(
                    "sflow_sfp_info",
                    "information about the SFP module",
                    1,
                    tags!(
                        "id" => id,
                        "total_lanes" => total_lanes,
                        "supply_voltage" => supply_voltage,
                    ),
                ),
                Metric::gauge_with_tags(
                    "sflow_sfp_temperature",
                    "temperature of the SFP module",
                    temperature,
                    tags!(
                        "id" => id,
                    ),
                ),
            ]);

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
                let tags = tags!(
                    "id" => id, // 1-based index of lane within module, 0=unknown
                    "index" => lane_index,
                );

                metrics.extend([
                    Metric::gauge_with_tags(
                        "sflow_sfp_tx_bias_current",
                        "value in microamps",
                        tx_bias_current,
                        tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "sflow_sfp_tx_power",
                        "in micro watts",
                        tx_power,
                        tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "sflow_sfp_tx_power_min",
                        "in micro watts",
                        tx_power_min,
                        tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "sflow_sfp_tx_power_max",
                        "in micro watts",
                        tx_power_max,
                        tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "sflow_sfp_tx_wavelength",
                        "in nano meters",
                        tx_wavelength,
                        tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "sflow_sfp_rx_power",
                        "in micro watts",
                        rx_power,
                        tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "sflow_sfp_rx_power_min",
                        "in micro watts",
                        rx_power_min,
                        tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "sflow_sfp_rx_power_max",
                        "in micro watts",
                        rx_power_max,
                        tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "sflow_sfp_rx_wavelength",
                        "in nano meters",
                        rx_wavelength,
                        tags,
                    ),
                ]);
            }

            metrics
        }
        CounterRecordData::VirtNode {
            mhz,
            cpus,
            memory,
            memory_free,
            num_domains,
        } => {
            vec![
                Metric::gauge("sflow_virt_node_mhz", "", mhz),
                Metric::gauge("sflow_virt_node_cpus", "", cpus),
                Metric::gauge("sflow_virt_node_memory", "", memory),
                Metric::gauge("sflow_virt_node_memory_free", "", memory_free),
                Metric::gauge("sflow_virt_node_num_domains", "", num_domains),
            ]
        }
        CounterRecordData::VirtCpu {
            state,
            cpu_time,
            nr_virt_cpu,
        } => {
            vec![
                Metric::gauge("sflow_virt_cpu_state", "", state),
                Metric::sum("sflow_virt_cpu_cpu_time_seconds", "", cpu_time / 1000),
                Metric::gauge("sflow_virt_cpu_total", "", nr_virt_cpu),
            ]
        }
        CounterRecordData::VirtMemory { memory, max_memory } => {
            vec![
                Metric::gauge("sflow_virt_memory_bytes", "", memory),
                Metric::gauge("sflow_virt_memory_max_bytes", "", max_memory),
            ]
        }
        CounterRecordData::VirtDisk {
            capacity,
            allocation,
            available,
            rd_req,
            rd_bytes,
            wr_req,
            wr_bytes,
            errs,
        } => {
            vec![
                Metric::gauge(
                    "sflow_virt_disk_capacity",
                    "logical size in bytes",
                    capacity,
                ),
                Metric::gauge(
                    "sflow_virt_disk_allocation",
                    "current allocation in bytes",
                    allocation,
                ),
                Metric::gauge(
                    "sflow_virt_disk_available",
                    "remaining free bytes",
                    available,
                ),
                Metric::gauge(
                    "sflow_virt_disk_read_req",
                    "number of read requests",
                    rd_req,
                ),
                Metric::gauge(
                    "sflow_virt_disk_read_bytes",
                    "number of read bytes",
                    rd_bytes,
                ),
                Metric::gauge(
                    "sflow_virt_disk_write_req",
                    "number of write requests",
                    wr_req,
                ),
                Metric::gauge(
                    "sflow_virt_disk_write_bytes",
                    "number of written bytes",
                    wr_bytes,
                ),
                Metric::gauge("sflow_virt_disk_errs", "read/write errors", errs),
            ]
        }
        CounterRecordData::VirtNetIO {
            rx_bytes,
            rx_packets,
            rx_errs,
            rx_drop,
            tx_bytes,
            tx_packets,
            tx_errs,
            tx_drop,
        } => {
            vec![
                Metric::sum("sflow_virt_net_rx_bytes", "total bytes received", rx_bytes),
                Metric::sum(
                    "sflow_virt_net_rx_packets",
                    "total packets received",
                    rx_packets,
                ),
                Metric::sum("sflow_virt_net_rx_errs", "total receive errors", rx_errs),
                Metric::sum("sflow_virt_net_rx_drop", "total receive drops", rx_drop),
                Metric::sum(
                    "sflow_virt_net_tx_bytes",
                    "total bytes transmitted",
                    tx_bytes,
                ),
                Metric::sum(
                    "sflow_virt_net_tx_packets",
                    "total packets transmitted",
                    tx_packets,
                ),
                Metric::sum("sflow_virt_net_tx_errs", "total transmit errors", tx_errs),
                Metric::sum("sflow_virt_net_tx_drop", "total transmit drops", tx_drop),
            ]
        }
        CounterRecordData::NvidiaGpu {
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
        } => {
            vec![
                Metric::gauge(
                    "sflow_nvidia_gpu_device_count",
                    "the number of accessible devices",
                    device_count,
                ),
                Metric::gauge(
                    "sflow_nvidia_gpu_processes",
                    "processes with a compute context on a device",
                    processes,
                ),
                Metric::sum(
                    "sflow_nvidia_gpu_time",
                    "total milliseconds in which one or more kernels was executing on GPU sum across all devices",
                    gpu_time,
                ),
                Metric::gauge(
                    "sflow_nvidia_gpu_mem_time",
                    "total milliseconds during which global device memory was being read/written sum across all devices",
                    mem_time,
                ),
                Metric::gauge(
                    "sflow_nvidia_gpu_mem_total",
                    "sum of framebuffer memory across devices",
                    mem_total,
                ),
                Metric::gauge(
                    "sflow_nvidia_gpu_mem_free",
                    "sum of free framebuffer memory across devices",
                    mem_free,
                ),
                Metric::gauge(
                    "sflow_nvidia_gpu_ecc_errors",
                    "sum of volatile ECC errors across devices",
                    ecc_errors,
                ),
                Metric::gauge(
                    "sflow_nvidia_gpu_energy",
                    "sum of millijoules across devices",
                    energy,
                ),
                Metric::gauge(
                    "sflow_nvidia_gpu_temperature",
                    "maximum temperature in degrees Celsius across devices",
                    temperature,
                ),
                Metric::gauge(
                    "sflow_nvidia_gpu_fan_speed",
                    "maximum fan speed in percent across devices",
                    fan_speed,
                ),
            ]
        }
        CounterRecordData::BcmTables {
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
        } => {
            vec![
                Metric::gauge("sflow_bcm_tables_host_entries", "", host_entries),
                Metric::gauge("sflow_bcm_tables_host_entries_max", "", host_entries_max),
                Metric::gauge("sflow_bcm_tables_ipv4_entries", "", ipv4_entries),
                Metric::gauge("sflow_bcm_tables_ipv4_entries_max", "", ipv4_entries_max),
                Metric::gauge("sflow_bcm_tables_ipv6_entries", "", ipv6_entries),
                Metric::gauge("sflow_bcm_tables_ipv6_entries_max", "", ipv6_entries_max),
                Metric::gauge("sflow_bcm_tables_ipv4_ipv6_entries", "", ipv4_ipv6_entries),
                Metric::gauge(
                    "sflow_bcm_tables_ipv4_ipv6_entries_max",
                    "",
                    ipv4_ipv6_entries_max,
                ),
                Metric::gauge("sflow_bcm_tables_long_ipv6_entries", "", long_ipv6_entries),
                Metric::gauge(
                    "sflow_bcm_tables_long_ipv6_entries_max",
                    "",
                    long_ipv6_entries_max,
                ),
                Metric::gauge("sflow_bcm_tables_total_routes", "", total_routes),
                Metric::gauge("sflow_bcm_tables_total_routes_max", "", total_routes_max),
                Metric::gauge("sflow_bcm_tables_ecmp_nexthops", "", ecmp_nexthops),
                Metric::gauge("sflow_bcm_tables_ecmp_nexthops_max", "", ecmp_nexthops_max),
                Metric::gauge("sflow_bcm_tables_mac_entries", "", mac_entries),
                Metric::gauge("sflow_bcm_tables_mac_entries_max", "", mac_entries_max),
                Metric::gauge("sflow_bcm_tables_ipv4_neighbors", "", ipv4_neighbors),
                Metric::gauge("sflow_bcm_tables_ipv6_neighbors", "", ipv6_neighbors),
                Metric::gauge("sflow_bcm_tables_ipv4_routes", "", ipv4_routes),
                Metric::gauge("sflow_bcm_tables_ipv6_routes", "", ipv6_routes),
                Metric::gauge(
                    "sflow_bcm_tables_acl_ingress_entries",
                    "",
                    acl_ingress_entries,
                ),
                Metric::gauge(
                    "sflow_bcm_tables_acl_ingress_entries_max",
                    "",
                    acl_ingress_entries_max,
                ),
                Metric::gauge(
                    "sflow_bcm_tables_acl_ingress_counters",
                    "",
                    acl_ingress_counters,
                ),
                Metric::gauge(
                    "sflow_bcm_tables_acl_ingress_counters_max",
                    "",
                    acl_ingress_counters_max,
                ),
                Metric::gauge(
                    "sflow_bcm_tables_acl_ingress_meters",
                    "",
                    acl_ingress_meters,
                ),
                Metric::gauge(
                    "sflow_bcm_tables_acl_ingress_meters_max",
                    "",
                    acl_ingress_meters_max,
                ),
                Metric::gauge(
                    "sflow_bcm_tables_acl_ingress_slices",
                    "",
                    acl_ingress_slices,
                ),
                Metric::gauge(
                    "sflow_bcm_tables_acl_ingress_slices_max",
                    "",
                    acl_ingress_slices_max,
                ),
                Metric::gauge(
                    "sflow_bcm_tables_acl_egress_entries",
                    "",
                    acl_egress_entries,
                ),
                Metric::gauge(
                    "sflow_bcm_tables_acl_egress_entries_max",
                    "",
                    acl_egress_entries_max,
                ),
                Metric::gauge(
                    "sflow_bcm_tables_acl_egress_counters",
                    "",
                    acl_egress_counters,
                ),
                Metric::gauge(
                    "sflow_bcm_tables_acl_egress_counters_max",
                    "",
                    acl_egress_counters_max,
                ),
                Metric::gauge("sflow_bcm_tables_acl_egress_meters", "", acl_egress_meters),
                Metric::gauge(
                    "sflow_bcm_tables_acl_egress_meters_max",
                    "",
                    acl_egress_meters_max,
                ),
                Metric::gauge("sflow_bcm_tables_acl_egress_slices", "", acl_egress_slices),
                Metric::gauge(
                    "sflow_bcm_tables_acl_egress_slices_max",
                    "",
                    acl_egress_slices_max,
                ),
            ]
        }
        CounterRecordData::Raw(format, ..) => {
            warn!(message = "unknown counter record type", format);

            vec![]
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    #[test]
    fn build_event() {
        let input = [
            0, 0, 0, 5, 0, 0, 0, 1, 192, 168, 88, 254, 0, 1, 134, 160, 0, 0, 1, 219, 0, 3, 105, 19,
            0, 0, 0, 1, 0, 0, 0, 4, 0, 0, 3, 52, 0, 0, 0, 45, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 11,
            0, 0, 8, 52, 0, 0, 0, 28, 0, 0, 14, 17, 0, 0, 0, 32, 0, 0, 0, 15, 171, 254, 128, 0, 0,
            0, 0, 1, 4, 241, 48, 0, 0, 0, 0, 43, 0, 0, 7, 209, 0, 0, 0, 116, 0, 0, 0, 7, 0, 0, 0,
            4, 0, 0, 0, 1, 2, 66, 201, 214, 193, 141, 8, 54, 0, 0, 0, 8, 0, 0, 0, 1, 2, 66, 81,
            122, 123, 221, 0, 15, 0, 0, 0, 2, 0, 0, 0, 1, 4, 217, 245, 249, 228, 34, 0, 1, 0, 0, 0,
            6, 0, 0, 0, 1, 2, 66, 230, 2, 129, 56, 0, 8, 0, 0, 0, 5, 0, 0, 0, 1, 2, 66, 202, 4,
            103, 211, 0, 76, 0, 0, 0, 3, 0, 0, 0, 1, 70, 192, 135, 254, 47, 40, 46, 115, 0, 0, 0,
            7, 0, 0, 0, 1, 2, 66, 20, 183, 202, 151, 90, 186, 0, 0, 7, 218, 0, 0, 0, 28, 1, 68,
            125, 72, 0, 0, 129, 69, 0, 0, 3, 228, 0, 182, 7, 0, 0, 0, 3, 228, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 7, 217, 0, 0, 0, 60, 0, 0, 0, 1, 0, 0, 0, 200, 0, 1, 212, 192, 255, 255, 255,
            255, 0, 55, 151, 97, 0, 1, 1, 99, 0, 50, 216, 139, 0, 0, 35, 104, 0, 0, 0, 103, 3, 84,
            99, 88, 3, 16, 107, 72, 0, 9, 165, 91, 0, 0, 2, 38, 0, 56, 77, 7, 0, 0, 0, 0, 0, 0, 7,
            216, 0, 0, 0, 100, 0, 1, 71, 180, 0, 0, 28, 223, 0, 0, 0, 0, 0, 1, 70, 114, 0, 0, 0,
            166, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 156, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 123, 53, 0, 0, 0, 0, 0, 0, 0, 124, 0, 0, 13, 213, 0,
            0, 122, 153, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 156,
            0, 0, 7, 215, 0, 0, 0, 76, 0, 0, 0, 1, 0, 0, 0, 64, 4, 139, 208, 36, 0, 0, 0, 0, 0, 0,
            0, 20, 0, 0, 0, 127, 0, 0, 0, 0, 0, 0, 0, 0, 4, 139, 126, 207, 3, 169, 247, 194, 0, 0,
            0, 0, 0, 1, 248, 95, 0, 0, 0, 0, 0, 0, 0, 21, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 7, 213, 0, 0, 0, 52, 0, 0, 1, 96, 25, 169, 16, 0, 0, 0, 0, 134,
            177, 237, 48, 0, 0, 0, 34, 116, 0, 135, 46, 207, 0, 0, 0, 69, 203, 148, 8, 0, 0, 57,
            132, 42, 6, 71, 116, 120, 0, 0, 2, 18, 8, 192, 168, 0, 94, 151, 113, 13, 0, 0, 7, 212,
            0, 0, 0, 72, 0, 0, 0, 15, 171, 254, 128, 0, 0, 0, 0, 1, 4, 241, 48, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 1, 32, 0, 0, 0, 0, 6, 150, 100, 160, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 8, 187, 25, 27, 66, 150, 245, 149, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 7, 211, 0, 0, 0, 80, 64, 249, 71, 174, 64, 211, 133, 31, 64, 171, 133, 31, 0, 0, 0,
            1, 0, 0, 30, 48, 0, 0, 0, 32, 0, 0, 14, 17, 0, 10, 156, 196, 33, 47, 211, 74, 0, 5,
            171, 134, 5, 219, 155, 236, 42, 91, 14, 206, 0, 166, 17, 132, 1, 235, 237, 222, 0, 243,
            243, 114, 108, 34, 126, 40, 48, 6, 1, 147, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7,
            214, 0, 0, 0, 40, 0, 0, 0, 0, 6, 249, 170, 170, 0, 1, 72, 86, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 64, 26, 133, 0, 0, 64, 251, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 208, 0, 0,
            0, 64, 0, 0, 0, 6, 102, 101, 100, 111, 114, 97, 0, 0, 26, 163, 85, 64, 167, 93, 120,
            125, 152, 156, 4, 217, 245, 249, 228, 34, 0, 0, 0, 3, 0, 0, 0, 2, 0, 0, 0, 22, 54, 46,
            49, 50, 46, 56, 45, 50, 48, 48, 46, 102, 99, 52, 49, 46, 120, 56, 54, 95, 54, 52, 0, 0,
        ];

        let (logs, metrics) = build_events(&input).unwrap();
        assert!(logs.is_empty());
        assert!(!metrics.is_empty());
    }
}
