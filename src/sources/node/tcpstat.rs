use std::path::{Path, PathBuf};
use netlink_packet_sock_diag::{
    NetlinkMessage,
    NetlinkHeader,
    NetlinkPayload,
    NLM_F_REQUEST,
    NLM_F_DUMP,
    SockDiagMessage,
    inet::{
        ExtensionFlags,
        InetRequest,
        SocketId,
        StateFlags,
    },
    constants::*,
};
use netlink_sys::{protocols::NETLINK_SOCK_DIAG, SocketAddr, TokioSocket};
use crate::event::Event;

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

pub async fn gather(root: PathBuf) -> Result<Vec<Event>, ()> {
    todo!()
}

async fn fetch_tcp_stats(family: u8) -> Statistics {
    let mut stats = Statistics::default();
    let mut socket = TokioSocket::new(NETLINK_SOCK_DIAG).unwrap();
    let _port = socket.bind_auto().unwrap().port_number();
    socket.connect(&SocketAddr::new(0, 0)).unwrap();

    let mut header = NetlinkHeader::default();
    header.flags = NLM_F_REQUEST | NLM_F_DUMP;
    let socket_id = match family {
        AF_INET => SocketId::new_v4(),
        AF_INET6 => SocketId::new_v6(),
        _ => panic!("unknown family")
    };

    let mut packet = NetlinkMessage {
        header,
        payload: SockDiagMessage::InetRequest(InetRequest {
            family,
            protocol: IPPROTO_TCP.into(),
            extensions: ExtensionFlags::empty(),
            states: StateFlags::all(),
            socket_id,
        }).into(),
    };

    packet.finalize();

    let mut buf = vec![0; packet.header.length as usize];
    assert_eq!(buf.len(), packet.buffer_len());
    packet.serialize(&mut buf[..]);

    if let Err(e) = socket.send(&buf[..]).await {
        panic!("send error {}", e);
    }

    let mut recv_buf = vec![0; 4096];
    let mut offset = 0;
    while let Ok(size) = socket.recv(&mut recv_buf).await {
        loop {
            let bytes = &recv_buf[offset..];
            let rx_packet = <NetlinkMessage<SockDiagMessage>>::deserialize(bytes).unwrap();

            match rx_packet.payload {
                NetlinkPayload::Noop | NetlinkPayload::Ack(_) => {}
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
                NetlinkPayload::Done => {
                    return stats;
                }
                NetlinkPayload::Error(_) | NetlinkPayload::Overrun(_) | _ => {
                    panic!("error or overrun")
                }
            }

            offset += rx_packet.header.length as usize;
            if offset == size || rx_packet.header.length == 0 {
                offset = 0;
                break;
            }
        }
    }

    stats
}

/*
#[tokio::main]
async fn main() {
    // the expressions are able to run concurrently but not in parallel
    let (v4, v6) = tokio::join!(
        fetch_tcp_stats(AF_INET),
        fetch_tcp_stats(AF_INET6),
    );

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

    println!("{:#?}", stats)
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use netlink_packet_sock_diag::{
        NetlinkMessage,
        NetlinkHeader,
        NetlinkPayload,
        NLM_F_REQUEST,
        NLM_F_DUMP,
        SockDiagMessage,
        inet::{
            ExtensionFlags,
            InetRequest,
            SocketId,
            StateFlags,
        },
    };
    use netlink_sys::{protocols::NETLINK_SOCK_DIAG, Socket, SocketAddr, TokioSocket};

    #[test]
    fn sync_inet_diag() {
        let mut socket = Socket::new(NETLINK_SOCK_DIAG).unwrap();
        let _port = socket.bind_auto().unwrap().port_number();
        socket.connect(&SocketAddr::new(0, 0)).unwrap();
        let mut stats = Statistics::default();

        let mut header = NetlinkHeader::default();
        header.flags = NLM_F_REQUEST | NLM_F_DUMP;

        let mut packet = NetlinkMessage {
            header,
            payload: SockDiagMessage::InetRequest(InetRequest {
                family: AF_INET,
                protocol: IPPROTO_TCP.into(),
                extensions: ExtensionFlags::empty(),
                states: StateFlags::all(),
                socket_id: SocketId::new_v4(),
            }).into(),
        };

        packet.finalize();

        let mut buf = vec![0; packet.header.length as usize];

        // Before calling serialize, it is important to check that the buffer in which we're
        // emitting is big enough for the packet, other `serialize()` panics.
        assert_eq!(buf.len(), packet.buffer_len());

        packet.serialize(&mut buf[..]);

        // println!(">>> {:?}", packet);
        if let Err(e) = socket.send(&buf[..], 0) {
            println!("SEND ERROR {}", e);
            return;
        }

        let mut recv_buf = vec![0; 4096];
        let mut offset = 0;
        while let Ok(size) = socket.recv(&mut recv_buf[..], 0) {
            loop {
                let bytes = &recv_buf[offset..];
                let rx_packet = <NetlinkMessage<SockDiagMessage>>::deserialize(bytes).unwrap();
                // println!("<<< {:?}", rx_packet);

                match rx_packet.payload {
                    NetlinkPayload::Noop | NetlinkPayload::Ack(_) => {}
                    NetlinkPayload::InnerMessage(SockDiagMessage::InetResponse(resp)) => {
                        // println!("{:#?}", resp);
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
                        /* println!("sock state {}", resp.header.state);
                         println!("rx_queue {} tx_queue {}",
                                  resp.header.recv_queue,
                                  resp.header.send_queue,
                         );*/
                    }
                    NetlinkPayload::Done => {
                        println!("Done");
                        println!("xxx {:#?}", stats);

                        return;
                    }
                    NetlinkPayload::Error(_) | NetlinkPayload::Overrun(_) | _ => return,
                }

                offset += rx_packet.header.length as usize;
                if offset == size || rx_packet.header.length == 0 {
                    offset = 0;
                    break;
                }
            }
        }
    }

    #[tokio::test]
    async fn async_inet_diag() {
        let mut stats = Statistics::default();
        let mut socket = TokioSocket::new(NETLINK_SOCK_DIAG).unwrap();
        let port = socket.bind_auto().unwrap().port_number();
        socket.connect(&SocketAddr::new(0, 0)).unwrap();

        let mut header = NetlinkHeader::default();
        header.flags = NLM_F_REQUEST | NLM_F_DUMP;

        let mut packet = NetlinkMessage {
            header,
            payload: SockDiagMessage::InetRequest(InetRequest {
                family: AF_INET,
                protocol: IPPROTO_TCP.into(),
                extensions: ExtensionFlags::empty(),
                states: StateFlags::all(),
                socket_id: SocketId::new_v4(),
            }).into(),
        };

        packet.finalize();

        let mut buf = vec![0; packet.header.length as usize];
        assert_eq!(buf.len(), packet.buffer_len());
        packet.serialize(&mut buf[..]);

        if let Err(e) = socket.send(&buf[..]).await {
            return;
        }

        let mut recv_buf = vec![0; 4096];
        let mut offset = 0;
        while let Ok(size) = socket.recv(&mut recv_buf).await {
            loop {
                let bytes = &recv_buf[offset..];
                let rx_packet = <NetlinkMessage<SockDiagMessage>>::deserialize(bytes).unwrap();

                match rx_packet.payload {
                    NetlinkPayload::Noop | NetlinkPayload::Ack(_) => {}
                    NetlinkPayload::InnerMessage(SockDiagMessage::InetResponse(resp)) => {
                        // println!("{:#?}", resp);
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
                    NetlinkPayload::Done => {
                        println!("Done");
                        println!("{:#?}", stats);

                        return;
                    }
                    NetlinkPayload::Error(_) | NetlinkPayload::Overrun(_) | _ => return,
                }

                offset += rx_packet.header.length as usize;
                if offset == size || rx_packet.header.length == 0 {
                    offset = 0;
                    break;
                }
            }
        }
    }
}