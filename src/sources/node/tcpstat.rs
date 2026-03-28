use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use event::{Metric, tags};
use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

use super::{Error, Paths};

#[derive(Default, Debug)]
struct Stats {
    established: usize,
    syn_sent: usize,
    syn_recv: usize,
    fin_wait1: usize,
    fin_wait2: usize,
    time_wait: usize,
    close: usize,
    close_wait: usize,
    last_ack: usize,
    listen: usize,
    closing: usize,

    rx_queued: usize,
    tx_queued: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct Header {
    pub length: u32,
    pub typ: u16,
    pub flags: u16,
    sequence: u32,
    pid: u32,
}

#[repr(C)]
struct InetDiagMsg {
    family: u8,
    state: u8,
    timer: u8,
    retrans: u8,
    id: [u8; 48],
    expires: u32,
    rx_queue: u32,
    tx_queue: u32,
    uid: u32,
    inode: u32,
}

async fn add_tcp_stats(
    sock: &mut NetlinkConnection,
    family: u8,
    stats: &mut Stats,
) -> io::Result<()> {
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

        // InetDiagReqV2 (inet_diag_req_v2) is used to request diagnostic data.
        // https://github.com/torvalds/linux/blob/v4.0/include/uapi/linux/inet_diag.h#L37
        family, 6, 2, 0,  // family, protocol, ext, pad
        255, 15, 0, 0,    // states TCPFAll
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0
    ];

    sock.write_all(&msg).await?;

    let mut buf = [0u8; 4096];
    loop {
        let count = sock.read(&mut buf).await?;

        // If this message is multi-part, we will need to continue looping
        // to drain all the messages from the socket
        let mut multi = false;

        // parse each message
        let mut offset = 0;
        while offset + NETLINK_HEADER_LEN < count {
            let header = unsafe { &*(buf.as_ptr().add(offset) as *const Header) };
            if header.length as usize + offset > count {
                return Err(io::Error::other("buf too short"));
            }

            let msg =
                unsafe { &*(buf.as_ptr().add(offset + NETLINK_HEADER_LEN) as *const InetDiagMsg) };
            stats.tx_queued += msg.tx_queue as usize;
            stats.rx_queued += msg.rx_queue as usize;

            if header.typ == SOCK_DIAG_BY_FAMILY {
                match msg.state {
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
                    state => warn!(
                        message = "unknown tcp state",
                        ?state,
                        internal_log_rate_limit = true
                    ),
                }
            }

            // Does this message indicate a multi-part messages?
            if header.flags & NLMSG_MULTI == 0 {
                // no, check the next messages
                continue;
            }

            // Does this message indicate the last message in a series of
            // multi-part messages from a single read?
            multi = header.typ != NLMSG_DONE;

            offset += header.length as usize;
        }

        if !multi {
            // no more messages coming
            break;
        }
    }

    Ok(())
}

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let mut sock = NetlinkConnection::connect()?;

    let mut stats = Stats::default();
    add_tcp_stats(&mut sock, libc::AF_INET as u8, &mut stats).await?;

    // if ipv6 system enabled
    if paths.proc().join("net/tcp6").exists() {
        add_tcp_stats(&mut sock, libc::AF_INET6 as u8, &mut stats).await?;
    }

    let mut metrics = Vec::with_capacity(15);
    for (state, value) in [
        ("established", stats.established),
        ("syn_sent", stats.syn_sent),
        ("syn_recv", stats.syn_recv),
        ("fin_wait1", stats.fin_wait1),
        ("fin_wait2", stats.fin_wait2),
        ("time_wait", stats.time_wait),
        ("close", stats.close),
        ("close_wait", stats.close_wait),
        ("last_ack", stats.last_ack),
        ("listen", stats.listen),
        ("closing", stats.closing),
        ("rx_queued_bytes", stats.rx_queued),
        ("tx_queued_bytes", stats.tx_queued),
    ] {
        if value == 0 {
            continue;
        }

        metrics.push(Metric::gauge_with_tags(
            "node_tcp_connection_states",
            "Number of connection states.",
            value,
            tags!("state" => state),
        ));
    }

    Ok(metrics)
}

/// Netlink Protocol type
const NETLINK_SOCK_DIAG: u16 = 4;
const SOCK_DIAG_BY_FAMILY: u16 = 20;

/// The message terminates a multipart message.
/// Data lost
const NLMSG_DONE: u16 = 3;
/// This type indicates a multi-part message, terminated by Done
/// on the last message.
const NLMSG_MULTI: u16 = 2;

/// (both server and client) represents an open connection, data
/// received can be delivered to the user. The normal state for the
/// data transfer phase of the connection.
const TCP_ESTABLISHED: u8 = 1;
/// (client) represents waiting for a matching connection request
/// after having sent a connection request.
const TCP_SYN_SENT: u8 = 2;
/// (server) represents waiting for a confirming connection request
/// acknowledgment after having both received and sent a connection
/// request.
const TCP_SYN_RECV: u8 = 3;
/// (both server and client) represents waiting for a connection
/// termination request from the remote TCP, or an acknowledgment of
/// the connection termination request previously sent.
const TCP_FIN_WAIT1: u8 = 4;
/// (both server and client) represents waiting for a connection
/// termination request from the remote TCP.
const TCP_FIN_WAIT2: u8 = 5;
/// (either server or client) represents waiting for enough time to
/// pass to be sure the remote TCP received the acknowledgment of its
/// connection termination request.
const TCP_TIME_WAIT: u8 = 6;
/// (both server and client) represents no connection state at all.
const TCP_CLOSE: u8 = 7;
/// (both server and client) represents waiting for a connection
/// termination request from the local user.
const TCP_CLOSE_WAIT: u8 = 8;
/// (both server and client) represents waiting for an acknowledgment
/// of the connection termination request previously sent to the
/// remote TCP (which includes an acknowledgment of its connection
/// termination request).
const TCP_LAST_ACK: u8 = 9;
/// (server) represents waiting for a connection request from any
/// remote TCP and port.
const TCP_LISTEN: u8 = 10;
/// (both server and client) represents waiting for a connection termination
/// request acknowledgment from the remote TCP.
const TCP_CLOSING: u8 = 11;

/// Length of a Netlink packet header.
const NETLINK_HEADER_LEN: usize = 16;

pub struct NetlinkConnection {
    inner: AsyncFd<OwnedFd>,
}

impl NetlinkConnection {
    pub fn _conn(family: u8) -> io::Result<Self> {
        let fd = unsafe {
            let ret = libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_DGRAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                family as libc::c_int,
            );

            if ret < 0 {
                return Err(io::Error::last_os_error());
            }

            OwnedFd::from_raw_fd(ret)
        };

        Ok(NetlinkConnection {
            inner: AsyncFd::new(fd)?,
        })
    }

    fn connect() -> io::Result<Self> {
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

        Ok(NetlinkConnection {
            inner: AsyncFd::new(fd)?,
        })
    }
}

impl AsyncRead for NetlinkConnection {
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

impl AsyncWrite for NetlinkConnection {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();
        assert!(!metrics.is_empty());
    }

    #[test]
    fn sizes() {
        assert_eq!(size_of::<Header>(), NETLINK_HEADER_LEN);
        assert_eq!(size_of::<InetDiagMsg>(), 72);
    }
}
