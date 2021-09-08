use netlink_packet_conntrack::{IPCTNL_MSG_CT_GET_STATS_CPU, IPCTNL_MSG_CT_GET_STATS, NetfilterRequest};
use netlink_sys::{Socket, SocketAddr};
use netlink_sys::constants::{NETLINK_SOCK_DIAG, NETLINK_NETFILTER};
use netlink_packet_core::{NetlinkHeader, NLM_F_REQUEST, NetlinkMessage, NetlinkPayload, NLM_F_DUMP};


fn main() {
    let mut socket = Socket::new(NETLINK_NETFILTER).unwrap();
    let _port = socket.bind_auto().unwrap().port_number();
    socket.connect(&SocketAddr::new(0, 0)).unwrap();

    let mut header = NetlinkHeader::default();
    header.flags = NLM_F_REQUEST | NLM_F_DUMP;

    let mut packet = NetlinkMessage {
        header,
        payload: NetfilterRequest::default().into(),
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

            let rxp = <NetlinkMessage<NetfilterRequest>>::deserialize(bytes).unwrap();
            match rxp.payload {
                NetlinkPayload::Noop | NetlinkPayload::Ack(_) => {}
                NetlinkPayload::Done => {
                    println!("done");
                    return;
                }

                NetlinkPayload::Error(err) => {
                    println!("err {}", err);
                    break;
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