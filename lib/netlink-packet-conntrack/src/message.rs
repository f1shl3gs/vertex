use netlink_packet_core::{
    NetlinkHeader, NLM_F_REQUEST, NetlinkMessage, NetlinkPayload,
    NetlinkDeserializable, DecodeError, NetlinkSerializable,
};
use netlink_sys::{
    SocketAddr, Socket,
    protocols::NETLINK_SOCK_DIAG,
};
use crate::{
    conntrack,
    traits::{Emitable, Parseable, ParseableParametrized}
};

pub enum NetfilterMessage {
    ConntrackRequest(conntrack::ConntrackRequest)
}

impl NetfilterMessage {
    pub fn message_type(&self) -> u16 {

    }
}

impl Emitable for NetfilterMessage {
    fn buffer_len(&self) -> usize {
        match self {
            Self::ConntrackRequest(ref msg) => msg.buffer_len(),
            _ => unreachable!()
        }
    }

    fn emit(&self, buf: &mut [u8]) {
        match self {
            Self::ConntrackRequest(ref msg) => msg.emit(buf),
            _ => unreachable!()
        }
    }
}

impl NetlinkDeserializable<NetfilterMessage> for NetfilterMessage {
    type Error = DecodeError;

    fn deserialize(header: &NetlinkHeader, payload: &[u8]) -> Result<NetfilterMessage, Self::Error> {
        let buf = NetfilterMessage::new_checkd(&payload)?;
        todo!()
    }
}