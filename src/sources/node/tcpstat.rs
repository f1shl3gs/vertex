use event::{tags, Metric};
use netlink_packet_core::{
    constants::{NLM_F_DUMP, NLM_F_REQUEST},
    NetlinkHeader, NetlinkMessage, NetlinkPayload,
};
use netlink_packet_sock_diag::{
    inet::{ExtensionFlags, InetRequest, SocketId, StateFlags},
    SockDiagMessage, AF_INET, AF_INET6, IPPROTO_TCP, TCP_CLOSE, TCP_CLOSE_WAIT, TCP_CLOSING,
    TCP_ESTABLISHED, TCP_FIN_WAIT1, TCP_FIN_WAIT2, TCP_LAST_ACK, TCP_LISTEN, TCP_SYN_RECV,
    TCP_SYN_SENT, TCP_TIME_WAIT,
};
use netlink_sys::{protocols::NETLINK_SOCK_DIAG, AsyncSocket, AsyncSocketExt, TokioSocket};

use super::Error;

#[derive(Default, Debug)]
struct Statistics {
    pub established: usize,
    pub syn_sent: usize,
    pub syn_recv: usize,
    pub fin_wait1: usize,
    pub fin_wait2: usize,
    pub time_wait: usize,
    pub close: usize,
    pub close_wait: usize,
    pub last_ack: usize,
    pub listen: usize,
    pub closing: usize,
}

macro_rules! state_metric {
    ($name: expr, $value: expr) => {
        Metric::gauge_with_tags(
            "node_tcp_connection_states",
            "",
            $value,
            tags!(
                "state" => $name
            )
        )
    };
}

pub async fn gather() -> Result<Vec<Metric>, Error> {
    let (v4, v6) = tokio::join!(fetch_tcp_stats(AF_INET), fetch_tcp_stats(AF_INET6),);

    let stats = Statistics {
        established: v4.established + v6.established,
        syn_sent: v4.syn_sent + v6.syn_sent,
        syn_recv: v4.syn_recv + v6.syn_recv,
        fin_wait1: v4.fin_wait1 + v6.fin_wait1,
        fin_wait2: v4.fin_wait2 + v6.fin_wait2,
        time_wait: v4.time_wait + v6.time_wait,
        close: v4.close + v6.close,
        close_wait: v4.close_wait + v6.close_wait,
        last_ack: v4.last_ack + v6.last_ack,
        listen: v4.listen + v6.listen,
        closing: v4.closing + v6.closing,
    };

    Ok(vec![
        state_metric!("established", stats.established),
        state_metric!("syn_sent", stats.syn_sent),
        state_metric!("syn_recv", stats.syn_recv),
        state_metric!("fin_wait1", stats.fin_wait1),
        state_metric!("fin_wait2", stats.fin_wait2),
        state_metric!("time_wait", stats.time_wait),
        state_metric!("close", stats.close),
        state_metric!("close_wait", stats.close_wait),
        state_metric!("last_ack", stats.last_ack),
        state_metric!("listen", stats.listen),
        state_metric!("closing", stats.closing),
    ])
}

async fn fetch_tcp_stats(family: u8) -> Statistics {
    let mut stats = Statistics::default();
    let mut socket = TokioSocket::new(NETLINK_SOCK_DIAG).unwrap();

    let mut header = NetlinkHeader::default();
    header.flags = NLM_F_REQUEST | NLM_F_DUMP;

    let socket_id = match family {
        AF_INET => SocketId::new_v4(),
        AF_INET6 => SocketId::new_v6(),
        _ => panic!("unknown family"),
    };

    let mut packet = NetlinkMessage::new(
        header,
        SockDiagMessage::InetRequest(InetRequest {
            family,
            protocol: IPPROTO_TCP,
            extensions: ExtensionFlags::empty(),
            states: StateFlags::all(),
            socket_id,
        })
        .into(),
    );

    packet.finalize();

    let mut buf = vec![0; packet.header.length as usize];
    assert_eq!(buf.len(), packet.buffer_len());
    packet.serialize(&mut buf[..]);

    if let Err(e) = socket.send(&buf[..]).await {
        panic!("send error {}", e);
    }

    let mut buf = vec![0; 8 * 1024];
    'RECV: loop {
        buf.clear();
        socket.recv(&mut buf).await.unwrap();

        let size = buf.len();
        let mut offset = 0;
        loop {
            let bytes = &buf[offset..];
            let pkt = <NetlinkMessage<SockDiagMessage>>::deserialize(bytes).unwrap();

            match pkt.payload {
                NetlinkPayload::Noop => {}
                NetlinkPayload::InnerMessage(SockDiagMessage::InetResponse(resp)) => {
                    match resp.header.state {
                        TCP_ESTABLISHED => stats.established += 1,
                        TCP_SYN_SENT => stats.syn_sent += 1,
                        TCP_SYN_RECV => stats.syn_recv += 1,
                        TCP_FIN_WAIT1 => stats.fin_wait1 += 1,
                        TCP_FIN_WAIT2 => stats.fin_wait2 += 1,
                        TCP_TIME_WAIT => stats.time_wait += 1,
                        TCP_CLOSE => stats.close += 1,
                        TCP_CLOSE_WAIT => stats.close_wait += 1,
                        TCP_LAST_ACK => stats.last_ack += 1,
                        TCP_LISTEN => stats.listen += 1,
                        TCP_CLOSING => stats.closing += 1,
                        _ => {}
                    }
                }
                NetlinkPayload::Done(_msg) => {
                    break 'RECV;
                }
                NetlinkPayload::Error(err) => {
                    error!(message = "Invalid netlink payload", ?err);
                    break 'RECV;
                }
                NetlinkPayload::Overrun(_) => {
                    panic!("overrun")
                }
                _ => {
                    panic!("Unknown payload type")
                }
            }

            offset += pkt.header.length as usize;
            if offset == size || pkt.header.length == 0 {
                break;
            }
        }
    }

    stats
}
