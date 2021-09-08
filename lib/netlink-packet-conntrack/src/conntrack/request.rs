use netlink_packet_core::{
    NetlinkHeader, NLM_F_REQUEST, NetlinkMessage, NetlinkPayload,
    NetlinkDeserializable, DecodeError, NetlinkSerializable,
};
use netlink_sys::{
    SocketAddr, Socket,
    protocols::NETLINK_SOCK_DIAG,
};

use crate::{traits::{Emitable, Parseable, ParseableParametrized}};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ConntrackRequest {
    pub family: u8,
    pub version: u8,
    pub res_id: u16,
}

impl Default for ConntrackRequest {
    fn default() -> Self {
        Self {
            family: 2, // 2 for AF_INET, 10 for AF_INET6
            version: 0,
            res_id: 0,
        }
    }
}

const REQUEST_LEN: usize = 4;

buffer!(ConntrackRequestBuffer(REQUEST_LEN) {
    family: (u8, 0),
    version: (u8, 1),
    resid: (u16, 2..4),
});

impl Emitable for ConntrackRequest {
    fn buffer_len(&self) -> usize {
        4
    }

    fn emit(&self, buf: &mut [u8]) {
        let mut buf = ConntrackRequestBuffer::new(buf);
        buf.set_family(self.family);
        buf.set_version(self.version);
        buf.set_resid(self.resid);
    }
}
