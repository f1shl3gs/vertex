use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::pin::Pin;
use std::task::{Context, Poll, ready};
use std::{io, mem};

use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncRead, ReadBuf};

pub struct Connection {
    fd: AsyncFd<OwnedFd>,
}

impl AsyncRead for Connection {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let conn = self.get_mut();

        loop {
            let mut guard = match ready!(conn.fd.poll_read_ready(cx)) {
                Ok(guard) => guard,
                Err(err) => return Poll::Ready(Err(err)),
            };

            let mut addr = unsafe { mem::zeroed::<libc::sockaddr_nl>() };
            let addr_ptr = &mut addr as *mut libc::sockaddr_nl as *mut libc::sockaddr;
            let mut addrlen = size_of_val(&addr);
            let addrlen_ptr = &mut addrlen as *mut usize as *mut libc::socklen_t;

            let unfilled = buf.initialize_unfilled();
            match guard.try_io(|inner| {
                let ret = unsafe {
                    libc::recvfrom(
                        inner.as_raw_fd(),
                        unfilled.as_mut_ptr() as *mut libc::c_void,
                        unfilled.len(),
                        0,
                        addr_ptr,
                        addrlen_ptr,
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
                Ok(Err(err)) => {
                    return Poll::Ready(Err(err));
                }
                Err(_would_block) => continue,
            }
        }
    }
}

impl Connection {
    pub fn connect() -> io::Result<Self> {
        let fd = unsafe {
            let ret = libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_RAW | libc::SOCK_CLOEXEC | libc::O_NONBLOCK,
                libc::NETLINK_AUDIT,
            );

            if ret < 0 {
                return Err(io::Error::last_os_error());
            }

            let mut addr = mem::zeroed::<libc::sockaddr_nl>();
            addr.nl_family = libc::AF_NETLINK as libc::sa_family_t;
            addr.nl_pid = 0;
            // "best effort" read only socket, defined in the kernel as AUDIT_NLGRP_READLOG
            addr.nl_groups = 1;

            let addr_ptr = &addr as *const libc::sockaddr_nl as *const libc::sockaddr;
            let addr_len = size_of::<libc::sockaddr_nl>() as libc::socklen_t;

            let res = libc::bind(ret, addr_ptr, addr_len);
            if res < 0 {
                let _ = libc::close(ret);

                return Err(io::Error::last_os_error());
            }

            OwnedFd::from_raw_fd(ret)
        };

        Ok(Connection {
            fd: AsyncFd::new(fd)?,
        })
    }
}
