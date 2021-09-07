use netlink_packet_core::{
    NetlinkHeader, NLM_F_REQUEST, NetlinkMessage,
    NetlinkPayload, NetlinkDeserializable,
    DecodeError, NetlinkSerializable,
};
use netlink_sys::{
    SocketAddr, Socket,
    protocols::NETLINK_SOCK_DIAG,
};
use netlink_packet_utils::{traits::Emitable};

use netlink_packet_conntrack::IPCTNL_MSG_CT_GET_STATS_CPU;

#[derive(Debug, PartialEq, Eq, Clone)]
struct NetfilterMessage {}

impl Emitable for NetfilterMessage {
    fn buffer_len(&self) -> usize {
        0
    }

    fn emit(&self, buffer: &mut [u8]) {}
}

impl From<NetfilterMessage> for NetlinkPayload<NetfilterMessage> {
    fn from(msg: NetfilterMessage) -> Self {
        NetlinkPayload::InnerMessage(msg)
    }
}

impl NetlinkDeserializable<NetfilterMessage> for NetfilterMessage {
    type Error = DecodeError;

    fn deserialize(header: &NetlinkHeader, payload: &[u8]) -> Result<NetfilterMessage, Self::Error> {
        println!("{:?}", header);

        todo!()
    }
}

impl NetlinkSerializable<NetfilterMessage> for NetfilterMessage {
    fn message_type(&self) -> u16 {
        // IPCTNL_MSG_CT_GET_STATS
        5u16
    }

    fn buffer_len(&self) -> usize {
        0
    }

    fn serialize(&self, buffer: &mut [u8]) {
        self.emit(buffer)
    }
}

fn main() {
    let mut socket = Socket::new(NETLINK_SOCK_DIAG).unwrap();
    let _port = socket.bind_auto().unwrap().port_number();
    socket.connect(&SocketAddr::new(0, 0)).unwrap();

    let mut header = NetlinkHeader::default();
    header.flags = NLM_F_REQUEST | IPCTNL_MSG_CT_GET_STATS_CPU;

    let mut packet = NetlinkMessage {
        header,
        payload: NetfilterMessage {}.into(),
    };

    packet.finalize();

    let mut buf = vec![0; packet.header.length as usize];
    assert_eq!(buf.len(), packet.buffer_len());

    packet.serialize(&mut buf[..]);

    if let Err(err) = socket.send(&buf[..], 0) {
        eprintln!("send error {}", err);
        return;
    }

    println!("send done");

    let mut recv_buf = vec![0; 4096];
    let mut offset = 0;
    while let Ok(size) = socket.recv(&mut recv_buf[..], 0) {
        loop {
            let bytes = &recv_buf[offset..];

            let rxp = <NetlinkMessage<NetfilterMessage>>::deserialize(bytes).unwrap();
            match rxp.payload {
                NetlinkPayload::Noop | NetlinkPayload::Ack(_) => {}
                NetlinkPayload::Done => {
                    println!("done");
                    return;
                }

                NetlinkPayload::Error(err) => {
                    println!("err {}", err);
                }

                NetlinkPayload::Overrun(or) => {
                    println!("overrun {:?}", or);
                }

                _ => {
                    println!("something else {:?} {:?}", rxp.header, rxp.payload);
                }
            }

            offset += rxp.header.length as usize;
            if offset == size || rxp.header.length == 0 {
                offset = 0;
                break;
            }
        }
    }
}