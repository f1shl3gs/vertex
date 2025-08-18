mod decode;
mod ipfix;
#[allow(clippy::module_inception)]
mod netflow;
mod template;

use std::net::Ipv6Addr;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::DerefMut;
use std::sync::Arc;

use configurable::configurable_component;
use decode::DataField;
use event::{Events, LogRecord};
use framework::config::{OutputType, Resource, SourceConfig, SourceContext};
use framework::source::udp::UdpSource;
use framework::{Error, Source};
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
    fn build_events(&self, peer: SocketAddr, data: &[u8]) -> Result<Events, Error> {
        let version = u16::from_be_bytes(data[0..2].try_into()?);

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

                convert_netflow(netflow)
            }
            _ => {
                warn!(
                    message = "invalid version of datagram",
                    %peer,
                    %version,
                    internal_log_rate_secs = 30
                );

                return Err(Error::from("unknown version"));
            }
        };

        let mut log = LogRecord::from(value);
        let metadata = log.metadata_mut().value_mut();

        metadata.insert("netflow.version", version);
        metadata.insert("netflow.peer", peer.to_string());

        Ok(log.into())
    }
}

fn convert_netflow(netflow: NetFlow) -> Value {
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

    value
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
            value.insert(
                "octetDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        2 => {
            value.insert(
                "packetDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        3 => {
            value.insert(
                "deltaFlowCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        4 => {
            value.insert("protocolIdentifier", field.data[0]);
        }
        5 => {
            value.insert("ipClassOfService", field.data[0]);
        }
        6 => {
            value.insert(
                "tcpControlBits",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        7 => {
            value.insert(
                "sourceTransportPort",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "ingressInterface",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        11 => {
            value.insert(
                "destinationTransportPort",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "egressInterface",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "bgpSourceAsNumber",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        17 => {
            value.insert(
                "bgpDestinationAsNumber",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "postMCastPacketDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        20 => {
            value.insert(
                "postMCastOctetDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        21 => {
            value.insert(
                "flowEndSysUpTime",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        22 => {
            value.insert(
                "flowStartSysUpTime",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        23 => {
            value.insert(
                "postOctetDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        24 => {
            value.insert(
                "postPacketDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        25 => {
            value.insert(
                "minimumIpTotalLength",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        26 => {
            value.insert(
                "maximumIpTotalLength",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        27 => {
            value.insert(
                "sourceIPv6Address",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
        }
        28 => {
            value.insert(
                "destinationIPv6Address",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
        }
        29 => {
            value.insert("sourceIPv6PrefixLength", field.data[0]);
        }
        30 => {
            value.insert("destinationIPv6PrefixLength", field.data[0]);
        }
        31 => {
            value.insert(
                "flowLabelIPv6",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        32 => {
            value.insert(
                "icmpTypeCodeIPv4",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        33 => {
            value.insert("igmpType", field.data[0]);
        }
        34 => {
            value.insert(
                "samplingInterval",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        35 => {
            value.insert("samplingAlgorithm", field.data[0]);
        }
        36 => {
            value.insert(
                "flowActiveTimeout",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        37 => {
            value.insert(
                "flowIdleTimeout",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        38 => {
            value.insert("engineType", field.data[0]);
        }
        39 => {
            value.insert("engineId", field.data[0]);
        }
        40 => {
            value.insert(
                "exportedOctetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        41 => {
            value.insert(
                "exportedMessageTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        42 => {
            value.insert(
                "exportedFlowRecordTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "samplerRandomInterval",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "fragmentIdentification",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "vlanId",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        59 => {
            value.insert(
                "postVlanId",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        60 => {
            value.insert("ipVersion", field.data[0]);
        }
        61 => {
            value.insert("flowDirection", field.data[0]);
        }
        62 => {
            value.insert(
                "ipNextHopIPv6Address",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
        }
        63 => {
            value.insert(
                "bgpNextHopIPv6Address",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
        }
        64 => {
            value.insert(
                "ipv6ExtensionHeaders",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert("interfaceName", String::from_utf8(field.data.into())?);
        }
        83 => {
            value.insert(
                "interfaceDescription",
                String::from_utf8(field.data.into())?,
            );
        }
        84 => {
            value.insert("samplerName", String::from_utf8(field.data.into())?);
        }
        85 => {
            value.insert(
                "octetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        86 => {
            value.insert(
                "packetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        87 => {
            value.insert(
                "flagsAndSamplerId",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        88 => {
            value.insert(
                "fragmentOffset",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        89 => {
            value.insert(
                "forwardingStatus",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        90 => {
            value.insert("mplsVpnRouteDistinguisher", field.data);
        }
        91 => {
            value.insert("mplsTopLabelPrefixLength", field.data[0]);
        }
        92 => {
            value.insert(
                "srcTrafficIndex",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        93 => {
            value.insert(
                "dstTrafficIndex",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        94 => {
            value.insert(
                "applicationDescription",
                String::from_utf8(field.data.into())?,
            );
        }
        95 => {
            value.insert("applicationId", field.data);
        }
        96 => {
            value.insert("applicationName", String::from_utf8(field.data.into())?);
        }
        98 => {
            value.insert("postIpDiffServCodePoint", field.data[0]);
        }
        99 => {
            value.insert(
                "multicastReplicationFactor",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        100 => {
            value.insert("className", String::from_utf8(field.data.into())?);
        }
        101 => {
            value.insert("classificationEngineId", field.data[0]);
        }
        102 => {
            value.insert(
                "layer2packetSectionOffset",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        103 => {
            value.insert(
                "layer2packetSectionSize",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        104 => {
            value.insert("layer2packetSectionData", field.data);
        }
        128 => {
            value.insert(
                "bgpNextAdjacentAsNumber",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        129 => {
            value.insert(
                "bgpPrevAdjacentAsNumber",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "exporterIPv6Address",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
        }
        132 => {
            value.insert(
                "droppedOctetDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        133 => {
            value.insert(
                "droppedPacketDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        134 => {
            value.insert(
                "droppedOctetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        135 => {
            value.insert(
                "droppedPacketTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        136 => {
            value.insert("flowEndReason", field.data[0]);
        }
        137 => {
            value.insert(
                "commonPropertiesId",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        138 => {
            value.insert(
                "observationPointId",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        139 => {
            value.insert(
                "icmpTypeCodeIPv6",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        140 => {
            value.insert(
                "mplsTopLabelIPv6Address",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
        }
        141 => {
            value.insert(
                "lineCardId",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        142 => {
            value.insert(
                "portId",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        143 => {
            value.insert(
                "meteringProcessId",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        144 => {
            value.insert(
                "exportingProcessId",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        145 => {
            value.insert(
                "templateId",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        146 => {
            value.insert("wlanChannelId", field.data[0]);
        }
        147 => {
            value.insert("wlanSSID", String::from_utf8(field.data.into())?);
        }
        148 => {
            value.insert(
                "flowId",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        149 => {
            value.insert(
                "observationDomainId",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        150 => {
            value.insert(
                "flowStartSeconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        151 => {
            value.insert(
                "flowEndSeconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        152 => {
            value.insert(
                "flowStartMilliseconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        153 => {
            value.insert(
                "flowEndMilliseconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        154 => {
            value.insert(
                "flowStartMicroseconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        155 => {
            value.insert(
                "flowEndMicroseconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        156 => {
            value.insert(
                "flowStartNanoseconds",
                i64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        157 => {
            value.insert(
                "flowEndNanoseconds",
                i64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        158 => {
            value.insert(
                "flowStartDeltaMicroseconds",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        159 => {
            value.insert(
                "flowEndDeltaMicroseconds",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        160 => {
            value.insert(
                "systemInitTimeMilliseconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        161 => {
            value.insert(
                "flowDurationMilliseconds",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        162 => {
            value.insert(
                "flowDurationMicroseconds",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        163 => {
            value.insert(
                "observedFlowTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        164 => {
            value.insert(
                "ignoredPacketTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        165 => {
            value.insert(
                "ignoredOctetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        166 => {
            value.insert(
                "notSentFlowTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        167 => {
            value.insert(
                "notSentPacketTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        168 => {
            value.insert(
                "notSentOctetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        169 => {
            value.insert(
                "destinationIPv6Prefix",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
        }
        170 => {
            value.insert(
                "sourceIPv6Prefix",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
        }
        171 => {
            value.insert(
                "postOctetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        172 => {
            value.insert(
                "postPacketTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        173 => {
            value.insert(
                "flowKeyIndicator",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        174 => {
            value.insert(
                "postMCastPacketTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        175 => {
            value.insert(
                "postMCastOctetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "udpSourcePort",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        181 => {
            value.insert(
                "udpDestinationPort",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        182 => {
            value.insert(
                "tcpSourcePort",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        183 => {
            value.insert(
                "tcpDestinationPort",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        184 => {
            value.insert(
                "tcpSequenceNumber",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        185 => {
            value.insert(
                "tcpAcknowledgementNumber",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        186 => {
            value.insert(
                "tcpWindowSize",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        187 => {
            value.insert(
                "tcpUrgentPointer",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        188 => {
            value.insert("tcpHeaderLength", field.data[0]);
        }
        189 => {
            value.insert("ipHeaderLength", field.data[0]);
        }
        190 => {
            value.insert(
                "totalLengthIPv4",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        191 => {
            value.insert(
                "payloadLengthIPv6",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        192 => {
            value.insert("ipTTL", field.data[0]);
        }
        193 => {
            value.insert("nextHeaderIPv6", field.data[0]);
        }
        194 => {
            value.insert(
                "mplsPayloadLength",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "octetDeltaSumOfSquares",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        199 => {
            value.insert(
                "octetTotalSumOfSquares",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        200 => {
            value.insert("mplsTopLabelTTL", field.data[0]);
        }
        201 => {
            value.insert(
                "mplsLabelStackLength",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        202 => {
            value.insert(
                "mplsLabelStackDepth",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        203 => {
            value.insert("mplsTopLabelExp", field.data[0]);
        }
        204 => {
            value.insert(
                "ipPayloadLength",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        205 => {
            value.insert(
                "udpMessageLength",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        206 => {
            value.insert("isMulticast", field.data[0]);
        }
        207 => {
            value.insert("ipv4IHL", field.data[0]);
        }
        208 => {
            value.insert(
                "ipv4Options",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        209 => {
            value.insert(
                "tcpOptions",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "collectorIPv6Address",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
        }
        213 => {
            value.insert(
                "exportInterface",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        214 => {
            value.insert("exportProtocolVersion", field.data[0]);
        }
        215 => {
            value.insert("exportTransportProtocol", field.data[0]);
        }
        216 => {
            value.insert(
                "collectorTransportPort",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        217 => {
            value.insert(
                "exporterTransportPort",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        218 => {
            value.insert(
                "tcpSynTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        219 => {
            value.insert(
                "tcpFinTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        220 => {
            value.insert(
                "tcpRstTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        221 => {
            value.insert(
                "tcpPshTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        222 => {
            value.insert(
                "tcpAckTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        223 => {
            value.insert(
                "tcpUrgTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        224 => {
            value.insert(
                "ipTotalLength",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "postNAPTSourceTransportPort",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        228 => {
            value.insert(
                "postNAPTDestinationTransportPort",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        229 => {
            value.insert("natOriginatingAddressRealm", field.data[0]);
        }
        230 => {
            value.insert("natEvent", field.data[0]);
        }
        231 => {
            value.insert(
                "initiatorOctets",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        232 => {
            value.insert(
                "responderOctets",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        233 => {
            value.insert("firewallEvent", field.data[0]);
        }
        234 => {
            value.insert(
                "ingressVRFID",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        235 => {
            value.insert(
                "egressVRFID",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        236 => {
            value.insert("VRFname", String::from_utf8(field.data.into())?);
        }
        237 => {
            value.insert("postMplsTopLabelExp", field.data[0]);
        }
        238 => {
            value.insert(
                "tcpWindowScale",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        239 => {
            value.insert("biflowDirection", field.data[0]);
        }
        240 => {
            value.insert("ethernetHeaderLength", field.data[0]);
        }
        241 => {
            value.insert(
                "ethernetPayloadLength",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        242 => {
            value.insert(
                "ethernetTotalLength",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        243 => {
            value.insert(
                "dot1qVlanId",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        244 => {
            value.insert("dot1qPriority", field.data[0]);
        }
        245 => {
            value.insert(
                "dot1qCustomerVlanId",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        246 => {
            value.insert("dot1qCustomerPriority", field.data[0]);
        }
        247 => {
            value.insert("metroEvcId", String::from_utf8(field.data.into())?);
        }
        248 => {
            value.insert("metroEvcType", field.data[0]);
        }
        249 => {
            value.insert(
                "pseudoWireId",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        250 => {
            value.insert(
                "pseudoWireType",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        251 => {
            value.insert(
                "pseudoWireControlWord",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        252 => {
            value.insert(
                "ingressPhysicalInterface",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        253 => {
            value.insert(
                "egressPhysicalInterface",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        254 => {
            value.insert(
                "postDot1qVlanId",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        255 => {
            value.insert(
                "postDot1qCustomerVlanId",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        256 => {
            value.insert(
                "ethernetType",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        257 => {
            value.insert("postIpPrecedence", field.data[0]);
        }
        258 => {
            value.insert(
                "collectionTimeMilliseconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        259 => {
            value.insert(
                "exportSctpStreamId",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        260 => {
            value.insert(
                "maxExportSeconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        261 => {
            value.insert(
                "maxFlowEndSeconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        262 => {
            value.insert("messageMD5Checksum", field.data);
        }
        263 => {
            value.insert("messageScope", field.data[0]);
        }
        264 => {
            value.insert(
                "minExportSeconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        265 => {
            value.insert(
                "minFlowStartSeconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        266 => {
            value.insert("opaqueOctets", field.data);
        }
        267 => {
            value.insert("sessionScope", field.data[0]);
        }
        268 => {
            value.insert(
                "maxFlowEndMicroseconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        269 => {
            value.insert(
                "maxFlowEndMilliseconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        270 => {
            value.insert(
                "maxFlowEndNanoseconds",
                i64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        271 => {
            value.insert(
                "minFlowStartMicroseconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        272 => {
            value.insert(
                "minFlowStartMilliseconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        273 => {
            value.insert(
                "minFlowStartNanoseconds",
                i64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "newConnectionDeltaCount",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        279 => {
            value.insert(
                "connectionSumDurationSeconds",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        280 => {
            value.insert(
                "connectionTransactionId",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        281 => {
            value.insert(
                "postNATSourceIPv6Address",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
        }
        282 => {
            value.insert(
                "postNATDestinationIPv6Address",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
        }
        283 => {
            value.insert(
                "natPoolId",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        284 => {
            value.insert("natPoolName", String::from_utf8(field.data.into())?);
        }
        285 => {
            value.insert(
                "anonymizationFlags",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        286 => {
            value.insert(
                "anonymizationTechnique",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        287 => {
            value.insert(
                "informationElementIndex",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        288 => {
            value.insert("p2pTechnology", String::from_utf8(field.data.into())?);
        }
        289 => {
            value.insert("tunnelTechnology", String::from_utf8(field.data.into())?);
        }
        290 => {
            value.insert("encryptedTechnology", String::from_utf8(field.data.into())?);
        }
        294 => {
            value.insert("bgpValidityState", field.data[0]);
        }
        295 => {
            value.insert(
                "IPSecSPI",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        296 => {
            value.insert(
                "greKey",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        297 => {
            value.insert("natType", field.data[0]);
        }
        298 => {
            value.insert(
                "initiatorPackets",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        299 => {
            value.insert(
                "responderPackets",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        300 => {
            value.insert(
                "observationDomainName",
                String::from_utf8(field.data.into())?,
            );
        }
        301 => {
            value.insert(
                "selectionSequenceId",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        302 => {
            value.insert(
                "selectorId",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        303 => {
            value.insert(
                "informationElementId",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        304 => {
            value.insert(
                "selectorAlgorithm",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        305 => {
            value.insert(
                "samplingPacketInterval",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        306 => {
            value.insert(
                "samplingPacketSpace",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        307 => {
            value.insert(
                "samplingTimeInterval",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        308 => {
            value.insert(
                "samplingTimeSpace",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        309 => {
            value.insert(
                "samplingSize",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        310 => {
            value.insert(
                "samplingPopulation",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        311 => {
            value.insert(
                "samplingProbability",
                f64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        312 => {
            value.insert(
                "dataLinkFrameSize",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "selectorIdTotalPktsObserved",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        319 => {
            value.insert(
                "selectorIdTotalPktsSelected",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        320 => {
            value.insert(
                "absoluteError",
                f64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        321 => {
            value.insert(
                "relativeError",
                f64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        322 => {
            value.insert(
                "observationTimeSeconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        323 => {
            value.insert(
                "observationTimeMilliseconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        324 => {
            value.insert(
                "observationTimeMicroseconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        325 => {
            value.insert(
                "observationTimeNanoseconds",
                i64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        326 => {
            value.insert(
                "digestHashValue",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        327 => {
            value.insert(
                "hashIPPayloadOffset",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        328 => {
            value.insert(
                "hashIPPayloadSize",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        329 => {
            value.insert(
                "hashOutputRangeMin",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        330 => {
            value.insert(
                "hashOutputRangeMax",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        331 => {
            value.insert(
                "hashSelectedRangeMin",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        332 => {
            value.insert(
                "hashSelectedRangeMax",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        333 => {
            value.insert("hashDigestOutput", field.data[0] == 1);
        }
        334 => {
            value.insert(
                "hashInitialiserValue",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        335 => {
            value.insert("selectorName", String::from_utf8(field.data.into())?);
        }
        336 => {
            value.insert(
                "upperCILimit",
                f64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        337 => {
            value.insert(
                "lowerCILimit",
                f64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        338 => {
            value.insert(
                "confidenceLevel",
                f64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        339 => {
            value.insert("informationElementDataType", field.data[0]);
        }
        340 => {
            value.insert(
                "informationElementDescription",
                String::from_utf8(field.data.into())?,
            );
        }
        341 => {
            value.insert(
                "informationElementName",
                String::from_utf8(field.data.into())?,
            );
        }
        342 => {
            value.insert(
                "informationElementRangeBegin",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        343 => {
            value.insert(
                "informationElementRangeEnd",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        344 => {
            value.insert("informationElementSemantics", field.data[0]);
        }
        345 => {
            value.insert(
                "informationElementUnits",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        346 => {
            value.insert(
                "privateEnterpriseNumber",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        347 => {
            value.insert("virtualStationInterfaceId", field.data);
        }
        348 => {
            value.insert(
                "virtualStationInterfaceName",
                String::from_utf8(field.data.into())?,
            );
        }
        349 => {
            value.insert("virtualStationUUID", field.data);
        }
        350 => {
            value.insert("virtualStationName", String::from_utf8(field.data.into())?);
        }
        351 => {
            value.insert(
                "layer2SegmentId",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        352 => {
            value.insert(
                "layer2OctetDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        353 => {
            value.insert(
                "layer2OctetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        354 => {
            value.insert(
                "ingressUnicastPacketTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        355 => {
            value.insert(
                "ingressMulticastPacketTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        356 => {
            value.insert(
                "ingressBroadcastPacketTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        357 => {
            value.insert(
                "egressUnicastPacketTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        358 => {
            value.insert(
                "egressBroadcastPacketTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        359 => {
            value.insert(
                "monitoringIntervalStartMilliSeconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        360 => {
            value.insert(
                "monitoringIntervalEndMilliSeconds",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        361 => {
            value.insert(
                "portRangeStart",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        362 => {
            value.insert(
                "portRangeEnd",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        363 => {
            value.insert(
                "portRangeStepSize",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        364 => {
            value.insert(
                "portRangeNumPorts",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "ingressInterfaceType",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        369 => {
            value.insert(
                "egressInterfaceType",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        370 => {
            value.insert(
                "rtpSequenceNumber",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        371 => {
            value.insert("userName", String::from_utf8(field.data.into())?);
        }
        372 => {
            value.insert(
                "applicationCategoryName",
                String::from_utf8(field.data.into())?,
            );
        }
        373 => {
            value.insert(
                "applicationSubCategoryName",
                String::from_utf8(field.data.into())?,
            );
        }
        374 => {
            value.insert(
                "applicationGroupName",
                String::from_utf8(field.data.into())?,
            );
        }
        375 => {
            value.insert(
                "originalFlowsPresent",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        376 => {
            value.insert(
                "originalFlowsInitiated",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        377 => {
            value.insert(
                "originalFlowsCompleted",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        378 => {
            value.insert(
                "distinctCountOfSourceIPAddress",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        379 => {
            value.insert(
                "distinctCountOfDestinationIPAddress",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        380 => {
            value.insert(
                "distinctCountOfSourceIPv4Address",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        381 => {
            value.insert(
                "distinctCountOfDestinationIPv4Address",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        382 => {
            value.insert(
                "distinctCountOfSourceIPv6Address",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        383 => {
            value.insert(
                "distinctCountOfDestinationIPv6Address",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        384 => {
            value.insert("valueDistributionMethod", field.data[0]);
        }
        385 => {
            value.insert(
                "rfc3550JitterMilliseconds",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        386 => {
            value.insert(
                "rfc3550JitterMicroseconds",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        387 => {
            value.insert(
                "rfc3550JitterNanoseconds",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        388 => {
            value.insert("dot1qDEI", field.data[0] == 1);
        }
        389 => {
            value.insert("dot1qCustomerDEI", field.data[0] == 1);
        }
        390 => {
            value.insert(
                "flowSelectorAlgorithm",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        391 => {
            value.insert(
                "flowSelectedOctetDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        392 => {
            value.insert(
                "flowSelectedPacketDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        393 => {
            value.insert(
                "flowSelectedFlowDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        394 => {
            value.insert(
                "selectorIDTotalFlowsObserved",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        395 => {
            value.insert(
                "selectorIDTotalFlowsSelected",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        396 => {
            value.insert(
                "samplingFlowInterval",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        397 => {
            value.insert(
                "samplingFlowSpacing",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        398 => {
            value.insert(
                "flowSamplingTimeInterval",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        399 => {
            value.insert(
                "flowSamplingTimeSpacing",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        400 => {
            value.insert(
                "hashFlowDomain",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        401 => {
            value.insert(
                "transportOctetDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        402 => {
            value.insert(
                "transportPacketDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "originalExporterIPv6Address",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
        }
        405 => {
            value.insert(
                "originalObservationDomainId",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        406 => {
            value.insert(
                "intermediateProcessId",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        407 => {
            value.insert(
                "ignoredDataRecordTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        408 => {
            value.insert(
                "dataLinkFrameType",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        409 => {
            value.insert(
                "sectionOffset",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        410 => {
            value.insert(
                "sectionExportedOctets",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        411 => {
            value.insert("dot1qServiceInstanceTag", field.data);
        }
        412 => {
            value.insert(
                "dot1qServiceInstanceId",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "postLayer2OctetDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        418 => {
            value.insert(
                "postMCastLayer2OctetDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        420 => {
            value.insert(
                "postLayer2OctetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        421 => {
            value.insert(
                "postMCastLayer2OctetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        422 => {
            value.insert(
                "minimumLayer2TotalLength",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        423 => {
            value.insert(
                "maximumLayer2TotalLength",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        424 => {
            value.insert(
                "droppedLayer2OctetDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        425 => {
            value.insert(
                "droppedLayer2OctetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        426 => {
            value.insert(
                "ignoredLayer2OctetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        427 => {
            value.insert(
                "notSentLayer2OctetTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        428 => {
            value.insert(
                "layer2OctetDeltaSumOfSquares",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        429 => {
            value.insert(
                "layer2OctetTotalSumOfSquares",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        430 => {
            value.insert(
                "layer2FrameDeltaCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        431 => {
            value.insert(
                "layer2FrameTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "ignoredLayer2FrameTotalCount",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        434 => {
            value.insert(
                "mibObjectValueInteger",
                i32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "mibObjectValueCounter",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        440 => {
            value.insert(
                "mibObjectValueGauge",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        441 => {
            value.insert(
                "mibObjectValueTimeTicks",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        442 => {
            value.insert(
                "mibObjectValueUnsigned",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        445 => {
            value.insert("mibObjectIdentifier", field.data);
        }
        446 => {
            value.insert(
                "mibSubIdentifier",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        447 => {
            value.insert(
                "mibIndexIndicator",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        448 => {
            value.insert("mibCaptureTimeSemantics", field.data[0]);
        }
        449 => {
            value.insert("mibContextEngineID", field.data);
        }
        450 => {
            value.insert("mibContextName", String::from_utf8(field.data.into())?);
        }
        451 => {
            value.insert("mibObjectName", String::from_utf8(field.data.into())?);
        }
        452 => {
            value.insert(
                "mibObjectDescription",
                String::from_utf8(field.data.into())?,
            );
        }
        453 => {
            value.insert("mibObjectSyntax", String::from_utf8(field.data.into())?);
        }
        454 => {
            value.insert("mibModuleName", String::from_utf8(field.data.into())?);
        }
        455 => {
            value.insert("mobileIMSI", String::from_utf8(field.data.into())?);
        }
        456 => {
            value.insert("mobileMSISDN", String::from_utf8(field.data.into())?);
        }
        457 => {
            value.insert(
                "httpStatusCode",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        458 => {
            value.insert(
                "sourceTransportPortsLimit",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        459 => {
            value.insert("httpRequestMethod", String::from_utf8(field.data.into())?);
        }
        460 => {
            value.insert("httpRequestHost", String::from_utf8(field.data.into())?);
        }
        461 => {
            value.insert("httpRequestTarget", String::from_utf8(field.data.into())?);
        }
        462 => {
            value.insert("httpMessageVersion", String::from_utf8(field.data.into())?);
        }
        463 => {
            value.insert(
                "natInstanceID",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        464 => {
            value.insert("internalAddressRealm", field.data);
        }
        465 => {
            value.insert("externalAddressRealm", field.data);
        }
        466 => {
            value.insert(
                "natQuotaExceededEvent",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        467 => {
            value.insert(
                "natThresholdEvent",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        468 => {
            value.insert("httpUserAgent", String::from_utf8(field.data.into())?);
        }
        469 => {
            value.insert("httpContentType", String::from_utf8(field.data.into())?);
        }
        470 => {
            value.insert("httpReasonPhrase", String::from_utf8(field.data.into())?);
        }
        471 => {
            value.insert(
                "maxSessionEntries",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        472 => {
            value.insert(
                "maxBIBEntries",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        473 => {
            value.insert(
                "maxEntriesPerUser",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        474 => {
            value.insert(
                "maxSubscribers",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        475 => {
            value.insert(
                "maxFragmentsPendingReassembly",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        476 => {
            value.insert(
                "addressPoolHighThreshold",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        477 => {
            value.insert(
                "addressPoolLowThreshold",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        478 => {
            value.insert(
                "addressPortMappingHighThreshold",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        479 => {
            value.insert(
                "addressPortMappingLowThreshold",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        480 => {
            value.insert(
                "addressPortMappingPerUserHighThreshold",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        481 => {
            value.insert(
                "globalAddressMappingHighThreshold",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        482 => {
            value.insert("vpnIdentifier", field.data);
        }
        483 => {
            value.insert(
                "bgpCommunity",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "srhTagIPv6",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        494 => {
            value.insert(
                "srhSegmentIPv6",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
        }
        495 => {
            value.insert(
                "srhActiveSegmentIPv6",
                Ipv6Addr::from(TryInto::<[u8; 16]>::try_into(field.data)?).to_string(),
            );
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
            value.insert(
                "srhSegmentIPv6EndpointBehavior",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        503 => {
            value.insert(
                "transportChecksum",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "gtpuTEid",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        508 => {
            value.insert(
                "gtpuSequenceNum",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
            value.insert(
                "ipv6ExtensionHeadersChainLength",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        521 => {
            value.insert(
                "tcpSharedOptionExID16",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        522 => {
            value.insert(
                "tcpSharedOptionExID32",
                u32::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        526 => {
            value.insert(
                "udpUnsafeOptions",
                u64::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
        }
        527 => {
            value.insert(
                "udpExID",
                u16::from_be_bytes(field.data.try_into().map_err(|_err| "invalid data value")?),
            );
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
