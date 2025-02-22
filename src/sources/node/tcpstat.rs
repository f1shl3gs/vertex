use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use event::{Metric, tags};
use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

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
            "Number of connection states.",
            $value,
            tags!(
                "state" => $name
            )
        )
    };
}

pub async fn gather() -> Result<Vec<Metric>, Error> {
    let v4 = fetch_tcp_stats(AF_INET).await?;
    let v6 = fetch_tcp_stats(AF_INET6).await?;

    Ok(vec![
        state_metric!("established", v4.established + v6.established),
        state_metric!("syn_sent", v4.syn_sent + v6.syn_sent),
        state_metric!("syn_recv", v4.syn_recv + v6.syn_recv),
        state_metric!("fin_wait1", v4.fin_wait1 + v6.fin_wait1),
        state_metric!("fin_wait2", v4.fin_wait2 + v6.fin_wait2),
        state_metric!("time_wait", v4.time_wait + v6.time_wait),
        state_metric!("close", v4.close + v6.close),
        state_metric!("close_wait", v4.close_wait + v6.close_wait),
        state_metric!("last_ack", v4.last_ack + v6.last_ack),
        state_metric!("listen", v4.listen + v6.listen),
        state_metric!("closing", v4.closing + v6.closing),
    ])
}

/// Netlink Protocol type
const NETLINK_SOCK_DIAG: u16 = 4;
const SOCK_DIAG_BY_FAMILY: u16 = 20;

/// The message is ignored.
pub const NLMSG_NOOP: u16 = 1;
/// The message signals an error and the payload contains a nlmsgerr structure.
/// This can be looked at as a NACK and typically it is from FEC to CPC.
pub const NLMSG_ERROR: u16 = 2;
/// The message terminates a multipart message.
/// Data lost
pub const NLMSG_DONE: u16 = 3;
pub const NLMSG_OVERRUN: u16 = 4;

const AF_INET: u8 = 2;
const AF_INET6: u8 = 10;

/// (both server and client) represents an open connection, data
/// received can be delivered to the user. The normal state for the
/// data transfer phase of the connection.
pub const TCP_ESTABLISHED: u8 = 1;
/// (client) represents waiting for a matching connection request
/// after having sent a connection request.
pub const TCP_SYN_SENT: u8 = 2;
/// (server) represents waiting for a confirming connection request
/// acknowledgment after having both received and sent a connection
/// request.
pub const TCP_SYN_RECV: u8 = 3;
/// (both server and client) represents waiting for a connection
/// termination request from the remote TCP, or an acknowledgment of
/// the connection termination request previously sent.
pub const TCP_FIN_WAIT1: u8 = 4;
/// (both server and client) represents waiting for a connection
/// termination request from the remote TCP.
pub const TCP_FIN_WAIT2: u8 = 5;
/// (either server or client) represents waiting for enough time to
/// pass to be sure the remote TCP received the acknowledgment of its
/// connection termination request.
pub const TCP_TIME_WAIT: u8 = 6;
/// (both server and client) represents no connection state at all.
pub const TCP_CLOSE: u8 = 7;
/// (both server and client) represents waiting for a connection
/// termination request from the local user.
pub const TCP_CLOSE_WAIT: u8 = 8;
/// (both server and client) represents waiting for an acknowledgment
/// of the connection termination request previously sent to the
/// remote TCP (which includes an acknowledgment of its connection
/// termination request).
pub const TCP_LAST_ACK: u8 = 9;
/// (server) represents waiting for a connection request from any
/// remote TCP and port.
pub const TCP_LISTEN: u8 = 10;
/// (both server and client) represents waiting for a connection termination
/// request acknowledgment from the remote TCP.
pub const TCP_CLOSING: u8 = 11;

/// Length of a Netlink packet header.
const NETLINK_HEADER_LEN: usize = 16;

struct NetlinkSocket {
    inner: AsyncFd<OwnedFd>,
}

impl NetlinkSocket {
    fn new() -> io::Result<Self> {
        let fd = unsafe {
            let ret = libc::socket(
                libc::PF_NETLINK,
                libc::SOCK_DGRAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                NETLINK_SOCK_DIAG as libc::c_int,
            );

            if ret < 0 {
                return Err(io::Error::last_os_error());
            }

            OwnedFd::from_raw_fd(ret)
        };

        Ok(NetlinkSocket {
            inner: AsyncFd::new(fd)?,
        })
    }
}

impl AsyncRead for NetlinkSocket {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            let mut guard = ready!(self.inner.poll_read_ready(cx))?;

            let unfilled = buf.initialize_unfilled();
            #[allow(clippy::blocks_in_conditions)]
            match guard.try_io(|inner| {
                let ret = unsafe {
                    libc::recv(
                        inner.as_raw_fd(),
                        unfilled.as_mut_ptr() as *mut libc::c_void,
                        unfilled.len(),
                        0,
                    )
                };
                if ret == -1 {
                    return Err(io::Error::last_os_error());
                }

                Ok(ret as usize)
            }) {
                Ok(Ok(len)) => {
                    buf.advance(len);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(err)) => return Poll::Ready(Err(err)),
                Err(_would_block) => continue,
            }
        }
    }
}

impl AsyncWrite for NetlinkSocket {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        loop {
            let mut guard = ready!(self.inner.poll_write_ready(cx))?;

            #[allow(clippy::blocks_in_conditions)]
            match guard.try_io(|inner| {
                let ret = unsafe {
                    libc::send(
                        inner.as_raw_fd(),
                        buf.as_ptr() as *const libc::c_void,
                        buf.len(),
                        libc::MSG_NOSIGNAL,
                    )
                };
                if ret < 0 {
                    return Err(io::Error::last_os_error());
                }

                Ok(ret as usize)
            }) {
                Ok(res) => return Poll::Ready(res),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        // Is netlink flush is a no-op !?
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let ret = unsafe { libc::shutdown(self.inner.as_raw_fd(), libc::SHUT_RDWR) };
        if ret == -1 {
            return Poll::Ready(Err(io::Error::last_os_error()));
        }

        Poll::Ready(Ok(()))
    }
}

async fn fetch_tcp_stats(family: u8) -> io::Result<Statistics> {
    #[rustfmt::skip]
    let msg = [
        // u32, length of the netlink packet, including the header and the payload
        72u8, 0, 0, 0,
        // u16, message type, SOCK_DIAG_BY_FAMILY
        20, 0,
        // u16, flags NLM_F_REQUEST | NLM_F_DUMP
        1, 3,
        // u32, sequence number
        1, 0, 0, 0,
        // u32, port number
        0, 0, 0, 0,

        // payload
        family, 6, 0, 0, 254, 15, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0
    ];

    let mut sock = NetlinkSocket::new()?;
    sock.write_all(&msg).await?;

    let mut stats = Statistics::default();

    let mut buf = [0u8; 8 * 1024];
    let mut read_offset = 0;
    'RECV: loop {
        let cnt = sock.read(&mut buf[read_offset..]).await? + read_offset;
        read_offset = 0;

        if cnt < 4 {
            read_offset = cnt;
            continue;
        }

        let mut offset = 0;
        while offset < cnt {
            let len = u32::from_ne_bytes((&buf[offset..offset + 4]).try_into().unwrap()) as usize;
            if cnt - offset < len {
                // not enough
                read_offset = cnt - offset;
                buf.copy_within(offset..cnt, 0);
                break;
            }

            // full packet
            let msg_typ = u16::from_ne_bytes((&buf[offset + 4..offset + 6]).try_into().unwrap());
            match msg_typ {
                SOCK_DIAG_BY_FAMILY => {
                    let state = match buf[offset + NETLINK_HEADER_LEN] {
                        AF_INET | AF_INET6 => buf[offset + NETLINK_HEADER_LEN + 1],
                        _family => continue,
                    };

                    match state {
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
                        _ => {
                            error!(message = "unknown tcp state", ?state);
                        }
                    }
                }
                NLMSG_NOOP => continue,
                NLMSG_DONE => break 'RECV,
                NLMSG_ERROR => return Err(io::Error::other("overrun packet")),
                _typ => return Err(io::Error::other("invalid packet type")),
            }

            offset += len;
        }

        if cnt < buf.len() {
            return Ok(stats);
        }
    }

    Ok(stats)
}
