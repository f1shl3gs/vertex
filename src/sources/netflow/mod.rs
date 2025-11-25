mod decode;
mod ipfix;
#[allow(clippy::module_inception)]
mod netflow;
mod template;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::DerefMut;
use std::sync::Arc;

use configurable::configurable_component;
use decode::{DataField, Error};
use event::{Events, LogRecord};
use framework::Source;
use framework::config::{OutputType, Resource, SourceConfig, SourceContext};
use framework::source::udp::UdpSource;
use ipfix::{DataRecord, FlowSet, IpFix, OptionsDataRecord};
use netflow::NetFlow;
use parking_lot::RwLock;
use template::{BasicTemplateSystem, TemplateSystem};
use value::Value;

fn default_listen() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 4739)
}

#[configurable_component(source, name = "netflow")]
struct Config {
    #[serde(default = "default_listen")]
    listen: SocketAddr,

    #[serde(default)]
    receive_buffer_size: Option<usize>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "netflow")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let source = NetFlowSource {
            templates: Arc::new(RwLock::new(BasicTemplateSystem::default())),
        };

        source.run(self.listen, self.receive_buffer_size, cx)
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::log()]
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::udp(self.listen)]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

struct NetFlowSource<T> {
    templates: Arc<RwLock<T>>,
}

impl<T> UdpSource for NetFlowSource<T>
where
    T: TemplateSystem + Send + Sync + 'static,
{
    type Error = Error;

    fn build_events(&self, peer: SocketAddr, data: &[u8]) -> Result<Events, Error> {
        if data.len() < 2 {
            return Err(Error::DatagramTooShort);
        }

        let version = u16::from_be_bytes(data[..2].try_into().unwrap());
        let value = match version {
            10 => {
                // IPFIX
                let ipfix = {
                    let mut templates = self.templates.write();
                    IpFix::decode(data, templates.deref_mut())?
                };

                convert_ipfix(ipfix)
            }
            9 => {
                // NetFlow v9
                let netflow = {
                    let mut templates = self.templates.write();
                    NetFlow::decode(data, templates.deref_mut())?
                };

                convert_netflow(netflow)?
            }
            version => {
                warn!(
                    message = "invalid version of datagram",
                    %peer,
                    version,
                    internal_log_rate_secs = 30
                );

                return Err(Error::IncompatibleVersion(version));
            }
        };

        let mut log = LogRecord::from(value);
        let metadata = log.metadata_mut().value_mut();

        metadata.insert("netflow.version", version);
        metadata.insert("netflow.peer", peer.to_string());

        Ok(log.into())
    }
}

fn convert_netflow(netflow: NetFlow) -> Result<Value, Error> {
    let mut value = Value::Object(Default::default());

    value.insert("version", netflow.version);
    value.insert("count", netflow.count);
    value.insert("system_uptime", netflow.system_uptime);
    value.insert("unix_seconds", netflow.unix_seconds);
    value.insert("sequence_number", netflow.sequence_number);
    value.insert("source_id", netflow.source_id);

    let mut flow_sets = Vec::with_capacity(netflow.flow_sets.len());
    for flow_set in netflow.flow_sets {
        let set = match flow_set {
            netflow::FlowSet::Data {
                template_id,
                length,
                records,
            } => {
                let mut value = Value::Object(Default::default());

                value.insert("template_id", template_id);
                value.insert("length", length);
                value.insert("records", convert_data_records(&records));

                value
            }
            netflow::FlowSet::OptionsData {
                template_id,
                records,
            } => {
                let mut value = Value::Object(Default::default());

                value.insert("template_id", template_id);
                value.insert("records", convert_options_data_records(&records));

                value
            }
            _ => continue,
        };

        flow_sets.push(set);
    }

    value.insert("flow_sets", flow_sets);

    Ok(value)
}

fn convert_ipfix(packet: IpFix) -> Value {
    let mut value = Value::Object(Default::default());

    value.insert("version", packet.version);
    value.insert("length", packet.length);
    value.insert("export_time", packet.export_time); // convert timestamp!?
    value.insert("sequence_number", packet.sequence_number);
    value.insert("observation_domain_id", packet.observation_domain_id);

    let mut flow_sets = Vec::with_capacity(packet.flow_sets.len());
    for flow_set in packet.flow_sets {
        let set = match flow_set {
            FlowSet::Data {
                template_id,
                length,
                records,
            } => {
                let mut set = Value::Object(Default::default());

                set.insert("template_id", template_id);
                set.insert("length", length);
                set.insert("records", convert_data_records(&records));

                set
            }
            FlowSet::OptionsData {
                template_id,
                length,
                records,
            } => {
                let mut set = Value::Object(Default::default());

                set.insert("template_id", template_id);
                set.insert("length", length);
                set.insert("records", convert_options_data_records(&records));

                set
            }
            _ => continue,
        };

        flow_sets.push(set);
    }

    value.insert("flow_sets", flow_sets);

    value
}

fn convert_data_records(records: &[DataRecord]) -> Vec<Value> {
    let mut array = Vec::with_capacity(records.len());

    for record in records {
        let mut value = Value::Object(Default::default());
        for field in &record.fields {
            let _ = set_property(&mut value, field);
        }

        array.push(value);
    }

    array
}

fn convert_options_data_records(records: &[OptionsDataRecord]) -> Vec<Value> {
    let mut array = Vec::with_capacity(records.len());

    for record in records {
        let mut value = Value::Object(Default::default());

        for field in &record.options {
            let _ = set_property(&mut value, field);
        }

        for field in &record.scopes {
            let _ = set_property(&mut value, field);
        }

        array.push(value);
    }

    array
}

/// https://www.iana.org/assignments/ipfix/ipfix.xhtml
fn set_property(value: &mut Value, field: &DataField) -> Result<(), Error> {
    match field.typ {
        1 => {
            value.insert("octetDeltaCount", field.to_u64()?);
        }
        2 => {
            value.insert("packetDeltaCount", field.to_u64()?);
        }
        3 => {
            value.insert("deltaFlowCount", field.to_u64()?);
        }
        4 => {
            value.insert("protocolIdentifier", field.data[0]);
        }
        5 => {
            value.insert("ipClassOfService", field.data[0]);
        }
        6 => {
            value.insert("tcpControlBits", field.to_u16()?);
        }
        7 => {
            value.insert("sourceTransportPort", field.to_u16()?);
        }
        8 => {
            value.insert(
                "sourceIPv4Address",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        9 => {
            value.insert("sourceIPv4PrefixLength", field.data[0]);
        }
        10 => {
            value.insert("ingressInterface", field.to_u32()?);
        }
        11 => {
            value.insert("destinationTransportPort", field.to_u16()?);
        }
        12 => {
            value.insert(
                "destinationIPv4Address",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        13 => {
            value.insert("destinationIPv4PrefixLength", field.data[0]);
        }
        14 => {
            value.insert("egressInterface", field.to_u32()?);
        }
        15 => {
            value.insert(
                "ipNextHopIPv4Address",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        16 => {
            value.insert("bgpSourceAsNumber", field.to_u32()?);
        }
        17 => {
            value.insert("bgpDestinationAsNumber", field.to_u32()?);
        }
        18 => {
            value.insert(
                "bgpNextHopIPv4Address",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        19 => {
            value.insert("postMCastPacketDeltaCount", field.to_u64()?);
        }
        20 => {
            value.insert("postMCastOctetDeltaCount", field.to_u64()?);
        }
        21 => {
            value.insert("flowEndSysUpTime", field.to_u32()?);
        }
        22 => {
            value.insert("flowStartSysUpTime", field.to_u32()?);
        }
        23 => {
            value.insert("postOctetDeltaCount", field.to_u64()?);
        }
        24 => {
            value.insert("postPacketDeltaCount", field.to_u64()?);
        }
        25 => {
            value.insert("minimumIpTotalLength", field.to_u64()?);
        }
        26 => {
            value.insert("maximumIpTotalLength", field.to_u64()?);
        }
        27 => {
            value.insert("sourceIPv6Address", field.ipv6()?.to_string());
        }
        28 => {
            value.insert("destinationIPv6Address", field.ipv6()?.to_string());
        }
        29 => {
            value.insert("sourceIPv6PrefixLength", field.data[0]);
        }
        30 => {
            value.insert("destinationIPv6PrefixLength", field.data[0]);
        }
        31 => {
            value.insert("flowLabelIPv6", field.to_u32()?);
        }
        32 => {
            value.insert("icmpTypeCodeIPv4", field.to_u16()?);
        }
        33 => {
            value.insert("igmpType", field.data[0]);
        }
        34 => {
            value.insert("samplingInterval", field.to_u32()?);
        }
        35 => {
            value.insert("samplingAlgorithm", field.data[0]);
        }
        36 => {
            value.insert("flowActiveTimeout", field.to_u16()?);
        }
        37 => {
            value.insert("flowIdleTimeout", field.to_u16()?);
        }
        38 => {
            value.insert("engineType", field.data[0]);
        }
        39 => {
            value.insert("engineId", field.data[0]);
        }
        40 => {
            value.insert("exportedOctetTotalCount", field.to_u64()?);
        }
        41 => {
            value.insert("exportedMessageTotalCount", field.to_u64()?);
        }
        42 => {
            value.insert("exportedFlowRecordTotalCount", field.to_u64()?);
        }
        43 => {
            value.insert(
                "ipv4RouterSc",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        44 => {
            value.insert(
                "sourceIPv4Prefix",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        45 => {
            value.insert(
                "destinationIPv4Prefix",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        46 => {
            value.insert("mplsTopLabelType", field.data[0]);
        }
        47 => {
            value.insert(
                "mplsTopLabelIPv4Address",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        48 => {
            value.insert("samplerId", field.data[0]);
        }
        49 => {
            value.insert("samplerMode", field.data[0]);
        }
        50 => {
            value.insert("samplerRandomInterval", field.to_u32()?);
        }
        51 => {
            value.insert("classId", field.data[0]);
        }
        52 => {
            value.insert("minimumTTL", field.data[0]);
        }
        53 => {
            value.insert("maximumTTL", field.data[0]);
        }
        54 => {
            value.insert("fragmentIdentification", field.to_u32()?);
        }
        55 => {
            value.insert("postIpClassOfService", field.data[0]);
        }
        56 => {
            value.insert(
                "sourceMacAddress",
                format!(
                    "{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}",
                    field.data[0],
                    field.data[1],
                    field.data[2],
                    field.data[3],
                    field.data[4],
                    field.data[5]
                ),
            );
        }
        57 => {
            value.insert(
                "postDestinationMacAddress",
                format!(
                    "{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}",
                    field.data[0],
                    field.data[1],
                    field.data[2],
                    field.data[3],
                    field.data[4],
                    field.data[5]
                ),
            );
        }
        58 => {
            value.insert("vlanId", field.to_u16()?);
        }
        59 => {
            value.insert("postVlanId", field.to_u16()?);
        }
        60 => {
            value.insert("ipVersion", field.data[0]);
        }
        61 => {
            value.insert("flowDirection", field.data[0]);
        }
        62 => {
            value.insert("ipNextHopIPv6Address", field.ipv6()?.to_string());
        }
        63 => {
            value.insert("bgpNextHopIPv6Address", field.ipv6()?.to_string());
        }
        64 => {
            value.insert("ipv6ExtensionHeaders", field.to_u32()?);
        }
        70 => {
            value.insert("mplsTopLabelStackSection", field.data);
        }
        71 => {
            value.insert("mplsLabelStackSection2", field.data);
        }
        72 => {
            value.insert("mplsLabelStackSection3", field.data);
        }
        73 => {
            value.insert("mplsLabelStackSection4", field.data);
        }
        74 => {
            value.insert("mplsLabelStackSection5", field.data);
        }
        75 => {
            value.insert("mplsLabelStackSection6", field.data);
        }
        76 => {
            value.insert("mplsLabelStackSection7", field.data);
        }
        77 => {
            value.insert("mplsLabelStackSection8", field.data);
        }
        78 => {
            value.insert("mplsLabelStackSection9", field.data);
        }
        79 => {
            value.insert("mplsLabelStackSection10", field.data);
        }
        80 => {
            value.insert(
                "destinationMacAddress",
                format!(
                    "{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}",
                    field.data[0],
                    field.data[1],
                    field.data[2],
                    field.data[3],
                    field.data[4],
                    field.data[5]
                ),
            );
        }
        81 => {
            value.insert(
                "postSourceMacAddress",
                format!(
                    "{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}",
                    field.data[0],
                    field.data[1],
                    field.data[2],
                    field.data[3],
                    field.data[4],
                    field.data[5]
                ),
            );
        }
        82 => {
            value.insert("interfaceName", field.string()?);
        }
        83 => {
            value.insert("interfaceDescription", field.string()?);
        }
        84 => {
            value.insert("samplerName", field.string()?);
        }
        85 => {
            value.insert("octetTotalCount", field.to_u64()?);
        }
        86 => {
            value.insert("packetTotalCount", field.to_u64()?);
        }
        87 => {
            value.insert("flagsAndSamplerId", field.to_u32()?);
        }
        88 => {
            value.insert("fragmentOffset", field.to_u16()?);
        }
        89 => {
            value.insert("forwardingStatus", field.to_u32()?);
        }
        90 => {
            value.insert("mplsVpnRouteDistinguisher", field.data);
        }
        91 => {
            value.insert("mplsTopLabelPrefixLength", field.data[0]);
        }
        92 => {
            value.insert("srcTrafficIndex", field.to_u32()?);
        }
        93 => {
            value.insert("dstTrafficIndex", field.to_u32()?);
        }
        94 => {
            value.insert("applicationDescription", field.string()?);
        }
        95 => {
            value.insert("applicationId", field.data);
        }
        96 => {
            value.insert("applicationName", field.string()?);
        }
        98 => {
            value.insert("postIpDiffServCodePoint", field.data[0]);
        }
        99 => {
            value.insert("multicastReplicationFactor", field.to_u32()?);
        }
        100 => {
            value.insert("className", field.string()?);
        }
        101 => {
            value.insert("classificationEngineId", field.data[0]);
        }
        102 => {
            value.insert("layer2packetSectionOffset", field.to_u16()?);
        }
        103 => {
            value.insert("layer2packetSectionSize", field.to_u16()?);
        }
        104 => {
            value.insert("layer2packetSectionData", field.data);
        }
        128 => {
            value.insert("bgpNextAdjacentAsNumber", field.to_u32()?);
        }
        129 => {
            value.insert("bgpPrevAdjacentAsNumber", field.to_u32()?);
        }
        130 => {
            value.insert(
                "exporterIPv4Address",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        131 => {
            value.insert("exporterIPv6Address", field.ipv6()?.to_string());
        }
        132 => {
            value.insert("droppedOctetDeltaCount", field.to_u64()?);
        }
        133 => {
            value.insert("droppedPacketDeltaCount", field.to_u64()?);
        }
        134 => {
            value.insert("droppedOctetTotalCount", field.to_u64()?);
        }
        135 => {
            value.insert("droppedPacketTotalCount", field.to_u64()?);
        }
        136 => {
            value.insert("flowEndReason", field.data[0]);
        }
        137 => {
            value.insert("commonPropertiesId", field.to_u64()?);
        }
        138 => {
            value.insert("observationPointId", field.to_u64()?);
        }
        139 => {
            value.insert("icmpTypeCodeIPv6", field.to_u16()?);
        }
        140 => {
            value.insert("mplsTopLabelIPv6Address", field.ipv6()?.to_string());
        }
        141 => {
            value.insert("lineCardId", field.to_u32()?);
        }
        142 => {
            value.insert("portId", field.to_u32()?);
        }
        143 => {
            value.insert("meteringProcessId", field.to_u32()?);
        }
        144 => {
            value.insert("exportingProcessId", field.to_u32()?);
        }
        145 => {
            value.insert("templateId", field.to_u16()?);
        }
        146 => {
            value.insert("wlanChannelId", field.data[0]);
        }
        147 => {
            value.insert("wlanSSID", field.string()?);
        }
        148 => {
            value.insert("flowId", field.to_u64()?);
        }
        149 => {
            value.insert("observationDomainId", field.to_u32()?);
        }
        150 => {
            value.insert("flowStartSeconds", field.to_i32()?);
        }
        151 => {
            value.insert("flowEndSeconds", field.to_i32()?);
        }
        152 => {
            value.insert("flowStartMilliseconds", field.to_i32()?);
        }
        153 => {
            value.insert("flowEndMilliseconds", field.to_i32()?);
        }
        154 => {
            value.insert("flowStartMicroseconds", field.to_i32()?);
        }
        155 => {
            value.insert("flowEndMicroseconds", field.to_i32()?);
        }
        156 => {
            value.insert("flowStartNanoseconds", field.to_i64()?);
        }
        157 => {
            value.insert("flowEndNanoseconds", field.to_i64()?);
        }
        158 => {
            value.insert("flowStartDeltaMicroseconds", field.to_u32()?);
        }
        159 => {
            value.insert("flowEndDeltaMicroseconds", field.to_u32()?);
        }
        160 => {
            value.insert("systemInitTimeMilliseconds", field.to_i32()?);
        }
        161 => {
            value.insert("flowDurationMilliseconds", field.to_u32()?);
        }
        162 => {
            value.insert("flowDurationMicroseconds", field.to_u32()?);
        }
        163 => {
            value.insert("observedFlowTotalCount", field.to_u64()?);
        }
        164 => {
            value.insert("ignoredPacketTotalCount", field.to_u64()?);
        }
        165 => {
            value.insert("ignoredOctetTotalCount", field.to_u64()?);
        }
        166 => {
            value.insert("notSentFlowTotalCount", field.to_u64()?);
        }
        167 => {
            value.insert("notSentPacketTotalCount", field.to_u64()?);
        }
        168 => {
            value.insert("notSentOctetTotalCount", field.to_u64()?);
        }
        169 => {
            value.insert("destinationIPv6Prefix", field.ipv6()?.to_string());
        }
        170 => {
            value.insert("sourceIPv6Prefix", field.ipv6()?.to_string());
        }
        171 => {
            value.insert("postOctetTotalCount", field.to_u64()?);
        }
        172 => {
            value.insert("postPacketTotalCount", field.to_u64()?);
        }
        173 => {
            value.insert("flowKeyIndicator", field.to_u64()?);
        }
        174 => {
            value.insert("postMCastPacketTotalCount", field.to_u64()?);
        }
        175 => {
            value.insert("postMCastOctetTotalCount", field.to_u64()?);
        }
        176 => {
            value.insert("icmpTypeIPv4", field.data[0]);
        }
        177 => {
            value.insert("icmpCodeIPv4", field.data[0]);
        }
        178 => {
            value.insert("icmpTypeIPv6", field.data[0]);
        }
        179 => {
            value.insert("icmpCodeIPv6", field.data[0]);
        }
        180 => {
            value.insert("udpSourcePort", field.to_u16()?);
        }
        181 => {
            value.insert("udpDestinationPort", field.to_u16()?);
        }
        182 => {
            value.insert("tcpSourcePort", field.to_u16()?);
        }
        183 => {
            value.insert("tcpDestinationPort", field.to_u16()?);
        }
        184 => {
            value.insert("tcpSequenceNumber", field.to_u32()?);
        }
        185 => {
            value.insert("tcpAcknowledgementNumber", field.to_u32()?);
        }
        186 => {
            value.insert("tcpWindowSize", field.to_u16()?);
        }
        187 => {
            value.insert("tcpUrgentPointer", field.to_u16()?);
        }
        188 => {
            value.insert("tcpHeaderLength", field.data[0]);
        }
        189 => {
            value.insert("ipHeaderLength", field.data[0]);
        }
        190 => {
            value.insert("totalLengthIPv4", field.to_u16()?);
        }
        191 => {
            value.insert("payloadLengthIPv6", field.to_u16()?);
        }
        192 => {
            value.insert("ipTTL", field.data[0]);
        }
        193 => {
            value.insert("nextHeaderIPv6", field.data[0]);
        }
        194 => {
            value.insert("mplsPayloadLength", field.to_u32()?);
        }
        195 => {
            value.insert("ipDiffServCodePoint", field.data[0]);
        }
        196 => {
            value.insert("ipPrecedence", field.data[0]);
        }
        197 => {
            value.insert("fragmentFlags", field.data[0]);
        }
        198 => {
            value.insert("octetDeltaSumOfSquares", field.to_u64()?);
        }
        199 => {
            value.insert("octetTotalSumOfSquares", field.to_u64()?);
        }
        200 => {
            value.insert("mplsTopLabelTTL", field.data[0]);
        }
        201 => {
            value.insert("mplsLabelStackLength", field.to_u32()?);
        }
        202 => {
            value.insert("mplsLabelStackDepth", field.to_u32()?);
        }
        203 => {
            value.insert("mplsTopLabelExp", field.data[0]);
        }
        204 => {
            value.insert("ipPayloadLength", field.to_u32()?);
        }
        205 => {
            value.insert("udpMessageLength", field.to_u16()?);
        }
        206 => {
            value.insert("isMulticast", field.data[0]);
        }
        207 => {
            value.insert("ipv4IHL", field.data[0]);
        }
        208 => {
            value.insert("ipv4Options", field.to_u32()?);
        }
        209 => {
            value.insert("tcpOptions", field.to_u64()?);
        }
        210 => {
            value.insert("paddingOctets", field.data);
        }
        211 => {
            value.insert(
                "collectorIPv4Address",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        212 => {
            value.insert("collectorIPv6Address", field.ipv6()?.to_string());
        }
        213 => {
            value.insert("exportInterface", field.to_u32()?);
        }
        214 => {
            value.insert("exportProtocolVersion", field.data[0]);
        }
        215 => {
            value.insert("exportTransportProtocol", field.data[0]);
        }
        216 => {
            value.insert("collectorTransportPort", field.to_u16()?);
        }
        217 => {
            value.insert("exporterTransportPort", field.to_u16()?);
        }
        218 => {
            value.insert("tcpSynTotalCount", field.to_u64()?);
        }
        219 => {
            value.insert("tcpFinTotalCount", field.to_u64()?);
        }
        220 => {
            value.insert("tcpRstTotalCount", field.to_u64()?);
        }
        221 => {
            value.insert("tcpPshTotalCount", field.to_u64()?);
        }
        222 => {
            value.insert("tcpAckTotalCount", field.to_u64()?);
        }
        223 => {
            value.insert("tcpUrgTotalCount", field.to_u64()?);
        }
        224 => {
            value.insert("ipTotalLength", field.to_u64()?);
        }
        225 => {
            value.insert(
                "postNATSourceIPv4Address",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        226 => {
            value.insert(
                "postNATDestinationIPv4Address",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        227 => {
            value.insert("postNAPTSourceTransportPort", field.to_u16()?);
        }
        228 => {
            value.insert("postNAPTDestinationTransportPort", field.to_u16()?);
        }
        229 => {
            value.insert("natOriginatingAddressRealm", field.data[0]);
        }
        230 => {
            value.insert("natEvent", field.data[0]);
        }
        231 => {
            value.insert("initiatorOctets", field.to_u64()?);
        }
        232 => {
            value.insert("responderOctets", field.to_u64()?);
        }
        233 => {
            value.insert("firewallEvent", field.data[0]);
        }
        234 => {
            value.insert("ingressVRFID", field.to_u32()?);
        }
        235 => {
            value.insert("egressVRFID", field.to_u32()?);
        }
        236 => {
            value.insert("VRFname", field.string()?);
        }
        237 => {
            value.insert("postMplsTopLabelExp", field.data[0]);
        }
        238 => {
            value.insert("tcpWindowScale", field.to_u16()?);
        }
        239 => {
            value.insert("biflowDirection", field.data[0]);
        }
        240 => {
            value.insert("ethernetHeaderLength", field.data[0]);
        }
        241 => {
            value.insert("ethernetPayloadLength", field.to_u16()?);
        }
        242 => {
            value.insert("ethernetTotalLength", field.to_u16()?);
        }
        243 => {
            value.insert("dot1qVlanId", field.to_u16()?);
        }
        244 => {
            value.insert("dot1qPriority", field.data[0]);
        }
        245 => {
            value.insert("dot1qCustomerVlanId", field.to_u16()?);
        }
        246 => {
            value.insert("dot1qCustomerPriority", field.data[0]);
        }
        247 => {
            value.insert("metroEvcId", field.string()?);
        }
        248 => {
            value.insert("metroEvcType", field.data[0]);
        }
        249 => {
            value.insert("pseudoWireId", field.to_u32()?);
        }
        250 => {
            value.insert("pseudoWireType", field.to_u16()?);
        }
        251 => {
            value.insert("pseudoWireControlWord", field.to_u32()?);
        }
        252 => {
            value.insert("ingressPhysicalInterface", field.to_u32()?);
        }
        253 => {
            value.insert("egressPhysicalInterface", field.to_u32()?);
        }
        254 => {
            value.insert("postDot1qVlanId", field.to_u16()?);
        }
        255 => {
            value.insert("postDot1qCustomerVlanId", field.to_u16()?);
        }
        256 => {
            value.insert("ethernetType", field.to_u16()?);
        }
        257 => {
            value.insert("postIpPrecedence", field.data[0]);
        }
        258 => {
            value.insert("collectionTimeMilliseconds", field.to_i32()?);
        }
        259 => {
            value.insert("exportSctpStreamId", field.to_u16()?);
        }
        260 => {
            value.insert("maxExportSeconds", field.to_i32()?);
        }
        261 => {
            value.insert("maxFlowEndSeconds", field.to_i32()?);
        }
        262 => {
            value.insert("messageMD5Checksum", field.data);
        }
        263 => {
            value.insert("messageScope", field.data[0]);
        }
        264 => {
            value.insert("minExportSeconds", field.to_i32()?);
        }
        265 => {
            value.insert("minFlowStartSeconds", field.to_i32()?);
        }
        266 => {
            value.insert("opaqueOctets", field.data);
        }
        267 => {
            value.insert("sessionScope", field.data[0]);
        }
        268 => {
            value.insert("maxFlowEndMicroseconds", field.to_i32()?);
        }
        269 => {
            value.insert("maxFlowEndMilliseconds", field.to_i32()?);
        }
        270 => {
            value.insert("maxFlowEndNanoseconds", field.to_i64()?);
        }
        271 => {
            value.insert("minFlowStartMicroseconds", field.to_i32()?);
        }
        272 => {
            value.insert("minFlowStartMilliseconds", field.to_i32()?);
        }
        273 => {
            value.insert("minFlowStartNanoseconds", field.to_i64()?);
        }
        274 => {
            value.insert("collectorCertificate", field.data);
        }
        275 => {
            value.insert("exporterCertificate", field.data);
        }
        276 => {
            value.insert("dataRecordsReliability", field.data[0] == 1);
        }
        277 => {
            value.insert("observationPointType", field.data[0]);
        }
        278 => {
            value.insert("newConnectionDeltaCount", field.to_u32()?);
        }
        279 => {
            value.insert("connectionSumDurationSeconds", field.to_u64()?);
        }
        280 => {
            value.insert("connectionTransactionId", field.to_u64()?);
        }
        281 => {
            value.insert("postNATSourceIPv6Address", field.ipv6()?.to_string());
        }
        282 => {
            value.insert("postNATDestinationIPv6Address", field.ipv6()?.to_string());
        }
        283 => {
            value.insert("natPoolId", field.to_u32()?);
        }
        284 => {
            value.insert("natPoolName", field.string()?);
        }
        285 => {
            value.insert("anonymizationFlags", field.to_u16()?);
        }
        286 => {
            value.insert("anonymizationTechnique", field.to_u16()?);
        }
        287 => {
            value.insert("informationElementIndex", field.to_u16()?);
        }
        288 => {
            value.insert("p2pTechnology", field.string()?);
        }
        289 => {
            value.insert("tunnelTechnology", field.string()?);
        }
        290 => {
            value.insert("encryptedTechnology", field.string()?);
        }
        294 => {
            value.insert("bgpValidityState", field.data[0]);
        }
        295 => {
            value.insert("IPSecSPI", field.to_u32()?);
        }
        296 => {
            value.insert("greKey", field.to_u32()?);
        }
        297 => {
            value.insert("natType", field.data[0]);
        }
        298 => {
            value.insert("initiatorPackets", field.to_u64()?);
        }
        299 => {
            value.insert("responderPackets", field.to_u64()?);
        }
        300 => {
            value.insert("observationDomainName", field.string()?);
        }
        301 => {
            value.insert("selectionSequenceId", field.to_u64()?);
        }
        302 => {
            value.insert("selectorId", field.to_u64()?);
        }
        303 => {
            value.insert("informationElementId", field.to_u16()?);
        }
        304 => {
            value.insert("selectorAlgorithm", field.to_u16()?);
        }
        305 => {
            value.insert("samplingPacketInterval", field.to_u32()?);
        }
        306 => {
            value.insert("samplingPacketSpace", field.to_u32()?);
        }
        307 => {
            value.insert("samplingTimeInterval", field.to_u32()?);
        }
        308 => {
            value.insert("samplingTimeSpace", field.to_u32()?);
        }
        309 => {
            value.insert("samplingSize", field.to_u32()?);
        }
        310 => {
            value.insert("samplingPopulation", field.to_u32()?);
        }
        311 => {
            value.insert("samplingProbability", field.to_f64()?);
        }
        312 => {
            value.insert("dataLinkFrameSize", field.to_u16()?);
        }
        313 => {
            value.insert("ipHeaderPacketSection", field.data);
        }
        314 => {
            value.insert("ipPayloadPacketSection", field.data);
        }
        315 => {
            value.insert("dataLinkFrameSection", field.data);
        }
        316 => {
            value.insert("mplsLabelStackSection", field.data);
        }
        317 => {
            value.insert("mplsPayloadPacketSection", field.data);
        }
        318 => {
            value.insert("selectorIdTotalPktsObserved", field.to_u64()?);
        }
        319 => {
            value.insert("selectorIdTotalPktsSelected", field.to_u64()?);
        }
        320 => {
            value.insert("absoluteError", field.to_f64()?);
        }
        321 => {
            value.insert("relativeError", field.to_f64()?);
        }
        322 => {
            value.insert("observationTimeSeconds", field.to_i32()?);
        }
        323 => {
            value.insert("observationTimeMilliseconds", field.to_i32()?);
        }
        324 => {
            value.insert("observationTimeMicroseconds", field.to_i32()?);
        }
        325 => {
            value.insert("observationTimeNanoseconds", field.to_i64()?);
        }
        326 => {
            value.insert("digestHashValue", field.to_u64()?);
        }
        327 => {
            value.insert("hashIPPayloadOffset", field.to_u64()?);
        }
        328 => {
            value.insert("hashIPPayloadSize", field.to_u64()?);
        }
        329 => {
            value.insert("hashOutputRangeMin", field.to_u64()?);
        }
        330 => {
            value.insert("hashOutputRangeMax", field.to_u64()?);
        }
        331 => {
            value.insert("hashSelectedRangeMin", field.to_u64()?);
        }
        332 => {
            value.insert("hashSelectedRangeMax", field.to_u64()?);
        }
        333 => {
            value.insert("hashDigestOutput", field.data[0] == 1);
        }
        334 => {
            value.insert("hashInitialiserValue", field.to_u64()?);
        }
        335 => {
            value.insert("selectorName", field.string()?);
        }
        336 => {
            value.insert("upperCILimit", field.to_f64()?);
        }
        337 => {
            value.insert("lowerCILimit", field.to_f64()?);
        }
        338 => {
            value.insert("confidenceLevel", field.to_f64()?);
        }
        339 => {
            value.insert("informationElementDataType", field.data[0]);
        }
        340 => {
            value.insert("informationElementDescription", field.string()?);
        }
        341 => {
            value.insert("informationElementName", field.string()?);
        }
        342 => {
            value.insert("informationElementRangeBegin", field.to_u64()?);
        }
        343 => {
            value.insert("informationElementRangeEnd", field.to_u64()?);
        }
        344 => {
            value.insert("informationElementSemantics", field.data[0]);
        }
        345 => {
            value.insert("informationElementUnits", field.to_u16()?);
        }
        346 => {
            value.insert("privateEnterpriseNumber", field.to_u32()?);
        }
        347 => {
            value.insert("virtualStationInterfaceId", field.data);
        }
        348 => {
            value.insert("virtualStationInterfaceName", field.string()?);
        }
        349 => {
            value.insert("virtualStationUUID", field.data);
        }
        350 => {
            value.insert("virtualStationName", field.string()?);
        }
        351 => {
            value.insert("layer2SegmentId", field.to_u64()?);
        }
        352 => {
            value.insert("layer2OctetDeltaCount", field.to_u64()?);
        }
        353 => {
            value.insert("layer2OctetTotalCount", field.to_u64()?);
        }
        354 => {
            value.insert("ingressUnicastPacketTotalCount", field.to_u64()?);
        }
        355 => {
            value.insert("ingressMulticastPacketTotalCount", field.to_u64()?);
        }
        356 => {
            value.insert("ingressBroadcastPacketTotalCount", field.to_u64()?);
        }
        357 => {
            value.insert("egressUnicastPacketTotalCount", field.to_u64()?);
        }
        358 => {
            value.insert("egressBroadcastPacketTotalCount", field.to_u64()?);
        }
        359 => {
            value.insert("monitoringIntervalStartMilliSeconds", field.to_i32()?);
        }
        360 => {
            value.insert("monitoringIntervalEndMilliSeconds", field.to_i32()?);
        }
        361 => {
            value.insert("portRangeStart", field.to_u16()?);
        }
        362 => {
            value.insert("portRangeEnd", field.to_u16()?);
        }
        363 => {
            value.insert("portRangeStepSize", field.to_u16()?);
        }
        364 => {
            value.insert("portRangeNumPorts", field.to_u16()?);
        }
        365 => {
            value.insert(
                "staMacAddress",
                format!(
                    "{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}",
                    field.data[0],
                    field.data[1],
                    field.data[2],
                    field.data[3],
                    field.data[4],
                    field.data[5]
                ),
            );
        }
        366 => {
            value.insert(
                "staIPv4Address",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        367 => {
            value.insert(
                "wtpMacAddress",
                format!(
                    "{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}",
                    field.data[0],
                    field.data[1],
                    field.data[2],
                    field.data[3],
                    field.data[4],
                    field.data[5]
                ),
            );
        }
        368 => {
            value.insert("ingressInterfaceType", field.to_u32()?);
        }
        369 => {
            value.insert("egressInterfaceType", field.to_u32()?);
        }
        370 => {
            value.insert("rtpSequenceNumber", field.to_u16()?);
        }
        371 => {
            value.insert("userName", field.string()?);
        }
        372 => {
            value.insert("applicationCategoryName", field.string()?);
        }
        373 => {
            value.insert("applicationSubCategoryName", field.string()?);
        }
        374 => {
            value.insert("applicationGroupName", field.string()?);
        }
        375 => {
            value.insert("originalFlowsPresent", field.to_u64()?);
        }
        376 => {
            value.insert("originalFlowsInitiated", field.to_u64()?);
        }
        377 => {
            value.insert("originalFlowsCompleted", field.to_u64()?);
        }
        378 => {
            value.insert("distinctCountOfSourceIPAddress", field.to_u64()?);
        }
        379 => {
            value.insert("distinctCountOfDestinationIPAddress", field.to_u64()?);
        }
        380 => {
            value.insert("distinctCountOfSourceIPv4Address", field.to_u32()?);
        }
        381 => {
            value.insert("distinctCountOfDestinationIPv4Address", field.to_u32()?);
        }
        382 => {
            value.insert("distinctCountOfSourceIPv6Address", field.to_u64()?);
        }
        383 => {
            value.insert("distinctCountOfDestinationIPv6Address", field.to_u64()?);
        }
        384 => {
            value.insert("valueDistributionMethod", field.data[0]);
        }
        385 => {
            value.insert("rfc3550JitterMilliseconds", field.to_u32()?);
        }
        386 => {
            value.insert("rfc3550JitterMicroseconds", field.to_u32()?);
        }
        387 => {
            value.insert("rfc3550JitterNanoseconds", field.to_u32()?);
        }
        388 => {
            value.insert("dot1qDEI", field.data[0] == 1);
        }
        389 => {
            value.insert("dot1qCustomerDEI", field.data[0] == 1);
        }
        390 => {
            value.insert("flowSelectorAlgorithm", field.to_u16()?);
        }
        391 => {
            value.insert("flowSelectedOctetDeltaCount", field.to_u64()?);
        }
        392 => {
            value.insert("flowSelectedPacketDeltaCount", field.to_u64()?);
        }
        393 => {
            value.insert("flowSelectedFlowDeltaCount", field.to_u64()?);
        }
        394 => {
            value.insert("selectorIDTotalFlowsObserved", field.to_u64()?);
        }
        395 => {
            value.insert("selectorIDTotalFlowsSelected", field.to_u64()?);
        }
        396 => {
            value.insert("samplingFlowInterval", field.to_u64()?);
        }
        397 => {
            value.insert("samplingFlowSpacing", field.to_u64()?);
        }
        398 => {
            value.insert("flowSamplingTimeInterval", field.to_u64()?);
        }
        399 => {
            value.insert("flowSamplingTimeSpacing", field.to_u64()?);
        }
        400 => {
            value.insert("hashFlowDomain", field.to_u16()?);
        }
        401 => {
            value.insert("transportOctetDeltaCount", field.to_u64()?);
        }
        402 => {
            value.insert("transportPacketDeltaCount", field.to_u64()?);
        }
        403 => {
            value.insert(
                "originalExporterIPv4Address",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        404 => {
            value.insert("originalExporterIPv6Address", field.ipv6()?.to_string());
        }
        405 => {
            value.insert("originalObservationDomainId", field.to_u32()?);
        }
        406 => {
            value.insert("intermediateProcessId", field.to_u32()?);
        }
        407 => {
            value.insert("ignoredDataRecordTotalCount", field.to_u64()?);
        }
        408 => {
            value.insert("dataLinkFrameType", field.to_u16()?);
        }
        409 => {
            value.insert("sectionOffset", field.to_u16()?);
        }
        410 => {
            value.insert("sectionExportedOctets", field.to_u16()?);
        }
        411 => {
            value.insert("dot1qServiceInstanceTag", field.data);
        }
        412 => {
            value.insert("dot1qServiceInstanceId", field.to_u32()?);
        }
        413 => {
            value.insert("dot1qServiceInstancePriority", field.data[0]);
        }
        414 => {
            value.insert(
                "dot1qCustomerSourceMacAddress",
                format!(
                    "{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}",
                    field.data[0],
                    field.data[1],
                    field.data[2],
                    field.data[3],
                    field.data[4],
                    field.data[5]
                ),
            );
        }
        415 => {
            value.insert(
                "dot1qCustomerDestinationMacAddress",
                format!(
                    "{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}",
                    field.data[0],
                    field.data[1],
                    field.data[2],
                    field.data[3],
                    field.data[4],
                    field.data[5]
                ),
            );
        }
        417 => {
            value.insert("postLayer2OctetDeltaCount", field.to_u64()?);
        }
        418 => {
            value.insert("postMCastLayer2OctetDeltaCount", field.to_u64()?);
        }
        420 => {
            value.insert("postLayer2OctetTotalCount", field.to_u64()?);
        }
        421 => {
            value.insert("postMCastLayer2OctetTotalCount", field.to_u64()?);
        }
        422 => {
            value.insert("minimumLayer2TotalLength", field.to_u64()?);
        }
        423 => {
            value.insert("maximumLayer2TotalLength", field.to_u64()?);
        }
        424 => {
            value.insert("droppedLayer2OctetDeltaCount", field.to_u64()?);
        }
        425 => {
            value.insert("droppedLayer2OctetTotalCount", field.to_u64()?);
        }
        426 => {
            value.insert("ignoredLayer2OctetTotalCount", field.to_u64()?);
        }
        427 => {
            value.insert("notSentLayer2OctetTotalCount", field.to_u64()?);
        }
        428 => {
            value.insert("layer2OctetDeltaSumOfSquares", field.to_u64()?);
        }
        429 => {
            value.insert("layer2OctetTotalSumOfSquares", field.to_u64()?);
        }
        430 => {
            value.insert("layer2FrameDeltaCount", field.to_u64()?);
        }
        431 => {
            value.insert("layer2FrameTotalCount", field.to_u64()?);
        }
        432 => {
            value.insert(
                "pseudoWireDestinationIPv4Address",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        433 => {
            value.insert("ignoredLayer2FrameTotalCount", field.to_u64()?);
        }
        434 => {
            value.insert("mibObjectValueInteger", field.to_i32()?);
        }
        435 => {
            value.insert("mibObjectValueOctetString", field.data);
        }
        436 => {
            value.insert("mibObjectValueOID", field.data);
        }
        437 => {
            value.insert("mibObjectValueBits", field.data);
        }
        438 => {
            value.insert(
                "mibObjectValueIPAddress",
                format!(
                    "{}.{}.{}.{}",
                    field.data[0], field.data[1], field.data[2], field.data[3]
                ),
            );
        }
        439 => {
            value.insert("mibObjectValueCounter", field.to_u64()?);
        }
        440 => {
            value.insert("mibObjectValueGauge", field.to_u32()?);
        }
        441 => {
            value.insert("mibObjectValueTimeTicks", field.to_u32()?);
        }
        442 => {
            value.insert("mibObjectValueUnsigned", field.to_u32()?);
        }
        445 => {
            value.insert("mibObjectIdentifier", field.data);
        }
        446 => {
            value.insert("mibSubIdentifier", field.to_u32()?);
        }
        447 => {
            value.insert("mibIndexIndicator", field.to_u64()?);
        }
        448 => {
            value.insert("mibCaptureTimeSemantics", field.data[0]);
        }
        449 => {
            value.insert("mibContextEngineID", field.data);
        }
        450 => {
            value.insert("mibContextName", field.string()?);
        }
        451 => {
            value.insert("mibObjectName", field.string()?);
        }
        452 => {
            value.insert("mibObjectDescription", field.string()?);
        }
        453 => {
            value.insert("mibObjectSyntax", field.string()?);
        }
        454 => {
            value.insert("mibModuleName", field.string()?);
        }
        455 => {
            value.insert("mobileIMSI", field.string()?);
        }
        456 => {
            value.insert("mobileMSISDN", field.string()?);
        }
        457 => {
            value.insert("httpStatusCode", field.to_u16()?);
        }
        458 => {
            value.insert("sourceTransportPortsLimit", field.to_u16()?);
        }
        459 => {
            value.insert("httpRequestMethod", field.string()?);
        }
        460 => {
            value.insert("httpRequestHost", field.string()?);
        }
        461 => {
            value.insert("httpRequestTarget", field.string()?);
        }
        462 => {
            value.insert("httpMessageVersion", field.string()?);
        }
        463 => {
            value.insert("natInstanceID", field.to_u32()?);
        }
        464 => {
            value.insert("internalAddressRealm", field.data);
        }
        465 => {
            value.insert("externalAddressRealm", field.data);
        }
        466 => {
            value.insert("natQuotaExceededEvent", field.to_u32()?);
        }
        467 => {
            value.insert("natThresholdEvent", field.to_u32()?);
        }
        468 => {
            value.insert("httpUserAgent", field.string()?);
        }
        469 => {
            value.insert("httpContentType", field.string()?);
        }
        470 => {
            value.insert("httpReasonPhrase", field.string()?);
        }
        471 => {
            value.insert("maxSessionEntries", field.to_u32()?);
        }
        472 => {
            value.insert("maxBIBEntries", field.to_u32()?);
        }
        473 => {
            value.insert("maxEntriesPerUser", field.to_u32()?);
        }
        474 => {
            value.insert("maxSubscribers", field.to_u32()?);
        }
        475 => {
            value.insert("maxFragmentsPendingReassembly", field.to_u32()?);
        }
        476 => {
            value.insert("addressPoolHighThreshold", field.to_u32()?);
        }
        477 => {
            value.insert("addressPoolLowThreshold", field.to_u32()?);
        }
        478 => {
            value.insert("addressPortMappingHighThreshold", field.to_u32()?);
        }
        479 => {
            value.insert("addressPortMappingLowThreshold", field.to_u32()?);
        }
        480 => {
            value.insert("addressPortMappingPerUserHighThreshold", field.to_u32()?);
        }
        481 => {
            value.insert("globalAddressMappingHighThreshold", field.to_u32()?);
        }
        482 => {
            value.insert("vpnIdentifier", field.data);
        }
        483 => {
            value.insert("bgpCommunity", field.to_u32()?);
        }
        486 => {
            value.insert("bgpExtendedCommunity", field.data);
        }
        489 => {
            value.insert("bgpLargeCommunity", field.data);
        }
        492 => {
            value.insert("srhFlagsIPv6", field.data[0]);
        }
        493 => {
            value.insert("srhTagIPv6", field.to_u16()?);
        }
        494 => {
            value.insert("srhSegmentIPv6", field.ipv6()?.to_string());
        }
        495 => {
            value.insert("srhActiveSegmentIPv6", field.ipv6()?.to_string());
        }
        497 => {
            value.insert("srhSegmentIPv6ListSection", field.data);
        }
        498 => {
            value.insert("srhSegmentsIPv6Left", field.data[0]);
        }
        499 => {
            value.insert("srhIPv6Section", field.data);
        }
        500 => {
            value.insert("srhIPv6ActiveSegmentType", field.data[0]);
        }
        501 => {
            value.insert("srhSegmentIPv6LocatorLength", field.data[0]);
        }
        502 => {
            value.insert("srhSegmentIPv6EndpointBehavior", field.to_u16()?);
        }
        503 => {
            value.insert("transportChecksum", field.to_u16()?);
        }
        504 => {
            value.insert("icmpHeaderPacketSection", field.data);
        }
        505 => {
            value.insert("gtpuFlags", field.data[0]);
        }
        506 => {
            value.insert("gtpuMsgType", field.data[0]);
        }
        507 => {
            value.insert("gtpuTEid", field.to_u32()?);
        }
        508 => {
            value.insert("gtpuSequenceNum", field.to_u16()?);
        }
        509 => {
            value.insert("gtpuQFI", field.data[0]);
        }
        510 => {
            value.insert("gtpuPduType", field.data[0]);
        }
        513 => {
            value.insert("ipv6ExtensionHeaderType", field.data[0]);
        }
        514 => {
            value.insert("ipv6ExtensionHeaderCount", field.data[0]);
        }
        517 => {
            value.insert("ipv6ExtensionHeadersLimit", field.data[0] == 1);
        }
        518 => {
            value.insert("ipv6ExtensionHeadersChainLength", field.to_u32()?);
        }
        521 => {
            value.insert("tcpSharedOptionExID16", field.to_u16()?);
        }
        522 => {
            value.insert("tcpSharedOptionExID32", field.to_u32()?);
        }
        526 => {
            value.insert("udpUnsafeOptions", field.to_u64()?);
        }
        527 => {
            value.insert("udpExID", field.to_u16()?);
        }

        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
