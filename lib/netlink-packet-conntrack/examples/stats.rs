use netlink_packet_core::{NetlinkHeader, NLM_F_REQUEST, NetlinkMessage, NetlinkPayload};
use netlink_sys::{
    SocketAddr, Socket,
    protocols::NETLINK_SOCK_DIAG,
};
use netlink_packet_conntrack::IPCTNL_MSG_CT_GET_STATS_CPU;

fn main() {
    let mut socket = Socket::new(NETLINK_SOCK_DIAG).unwrap();
    let _port = socket.bind_auto().unwrap().port_number();
    socket.connect(&SocketAddr::new(0, 0)).unwrap();

    let mut header = NetlinkHeader::default();
    header.flags = NLM_F_REQUEST | IPCTNL_MSG_CT_GET_STATS_CPU;

    let mut packet = NetlinkMessage {
        header,
        payload: NetlinkPayload::Noop,
    };

    packet.finalize();

    let mut buf = vec![0; packet.header.length as usize];
    assert_eq!(buf.len(), packet.buffer_len());

    packet.serialize(&mut buf[..]);

    if let Err(err) = socket.send(&buf[..], 0) {
        eprintln!("send error {}", err);
        return;
    }

    let mut recv_buf = vec![0; 4096];
    let mut offset = 0;
    while let Ok(size) = socket.recv(&mut recv_buf[..], 0) {
        loop {
            let bytes = &recv_buf[offset..];

            // todo
        }
    }
}