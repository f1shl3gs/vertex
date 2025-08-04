use std::alloc::Layout;
use std::ffi::CString;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::task::{Poll, Waker, ready};

use futures::task::AtomicWaker;
use tokio::io::unix::AsyncFd;

pub struct Registration {
    fd: RawFd, // inotify fd

    capacity: usize,
    // the underlying watch descriptor storage in the kernel is an `idr`, an array which
    // can be access with an index.
    //
    // https://github.com/torvalds/linux/blob/f4a40a4282f467ec99745c6ba62cb84346e42139/include/linux/idr.h#L20
    slots: *mut AtomicWaker,
}

unsafe impl Send for Registration {}

impl Drop for Registration {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::array::<AtomicWaker>(self.capacity + 1).unwrap();
            std::alloc::dealloc(self.slots as *mut _, layout);
        }
    }
}

impl Registration {
    pub fn new(capacity: usize) -> io::Result<(Self, EventStream)> {
        let fd = unsafe {
            let ret = libc::inotify_init1(libc::O_CLOEXEC | libc::O_NONBLOCK);
            if ret == -1 {
                return Err(io::Error::last_os_error());
            }

            OwnedFd::from_raw_fd(ret)
        };

        let slots = unsafe {
            // wd start from 1, so later we don't need to sub 1 to access waker
            let layout = Layout::array::<AtomicWaker>(capacity + 1).unwrap();
            std::alloc::alloc_zeroed(layout) as *mut AtomicWaker
        };

        let registration = Registration {
            fd: fd.as_raw_fd(),
            capacity,
            slots,
        };
        let stream = EventStream {
            fd: AsyncFd::new(fd)?,
            capacity,
            slots,
        };

        Ok((registration, stream))
    }

    pub fn add(&self, path: &Path) -> io::Result<Handle> {
        let path = CString::new(path.as_os_str().as_bytes()).map_err(io::Error::other)?;

        let wd = unsafe {
            let ret = libc::inotify_add_watch(
                self.fd,
                path.as_ptr() as *const _,
                // libc::IN_MOVE | libc::IN_MODIFY | libc::IN_Q_OVERFLOW,
                libc::IN_DELETE
                    | libc::IN_IGNORED
                    | libc::IN_MODIFY
                    | libc::IN_MOVE_SELF
                    | libc::IN_Q_OVERFLOW,
            );
            if ret == -1 {
                return Err(io::Error::last_os_error());
            }

            ret as RawFd
        };

        Ok(Handle {
            fd: self.fd,
            wd,
            waker: unsafe { self.slots.add(wd as usize) },
        })
    }

    /*
    pub fn remove(&self, id: Id) -> io::Result<()> {
        unsafe {
            let ret = libc::inotify_rm_watch(self.fd, id as i32);
            if ret == -1 {
                return Err(io::Error::last_os_error());
            }
        }

        Ok(())
    }
    */
}

pub struct EventStream {
    fd: AsyncFd<OwnedFd>,

    capacity: usize,
    slots: *mut AtomicWaker,
}

unsafe impl Send for EventStream {}
unsafe impl Sync for EventStream {}

impl EventStream {
    pub async fn wait_and_wake(&self) -> io::Result<()> {
        const EVENT_SIZE: usize = 4 * size_of::<u32>();

        let buf = [0u32; 256 * EVENT_SIZE];

        let count = futures::future::poll_fn(|cx| {
            loop {
                let mut guard = ready!(self.fd.poll_read_ready(cx))?;

                match guard.try_io(|inner| {
                    let ret = unsafe {
                        libc::read(inner.as_raw_fd(), buf.as_ptr() as *mut _, size_of_val(&buf))
                    };
                    if ret == -1 {
                        return Err(io::Error::last_os_error());
                    }

                    Ok(ret as usize)
                }) {
                    Ok(Ok(len)) => {
                        return Poll::Ready(Ok(len / EVENT_SIZE));
                    }
                    Ok(Err(err)) => return Poll::Ready(Err(err)),
                    Err(_would_block) => continue,
                }
            }
        })
        .await?;

        unsafe {
            // More info: https://www.man7.org/linux/man-pages/man7/inotify.7.html
            //
            // struct inotify_event {
            //     int      wd;       /* Watch descriptor */
            //     uint32_t mask;     /* Mask describing event */
            //     uint32_t cookie;   /* Unique cookie associating related
            //                          events (for rename(2)) */
            //     uint32_t len;      /* Size of name field */
            //     char     name[];   /* Optional null-terminated name */
            // };

            // NOTE: maybe we can debounce here
            for index in 0..count {
                let wd = buf[index * 4] as usize;
                let mask = buf[index * 4 + 1];

                if mask & libc::IN_Q_OVERFLOW == libc::IN_Q_OVERFLOW {
                    // Event queue overflowed, just wake all
                    for index in 1..self.capacity {
                        (&*self.slots.add(index)).wake();
                    }

                    return Ok(());
                }

                (&*self.slots.add(wd)).wake();
            }
        }

        Ok(())
    }
}

/*
#[cfg(test)]
fn print_event(wd: u32, mask: u32) {
    if mask & libc::IN_ACCESS == libc::IN_ACCESS {
        println!("{wd} -- ACCESS");
    }

    if mask & libc::IN_MODIFY == libc::IN_MODIFY {
        println!("{wd} -- MODIFY");
    }

    if mask & libc::IN_ATTRIB == libc::IN_ATTRIB {
        println!("{wd} -- ATTRIB");
    }

    if mask & libc::IN_CLOSE_WRITE == libc::IN_CLOSE_WRITE {
        println!("{wd} -- CLOSE_WRITE");
    }

    if mask & libc::IN_CLOSE_NOWRITE == libc::IN_CLOSE_NOWRITE {
        println!("{wd} -- CLOSE_NOWRITE");
    }

    if mask & libc::IN_OPEN == libc::IN_OPEN {
        println!("{wd} -- OPEN");
    }

    if mask & libc::IN_MOVED_FROM == libc::IN_MOVED_FROM {
        println!("{wd} -- MOVED_FROM");
    }

    if mask & libc::IN_MOVED_TO == libc::IN_MOVED_TO {
        println!("{wd} -- MOVED_TO");
    }

    if mask & libc::IN_CREATE == libc::IN_CREATE {
        println!("{wd} -- CREATE");
    }

    if mask & libc::IN_DELETE == libc::IN_DELETE {
        println!("{wd} -- DELETE");
    }

    if mask & libc::IN_DELETE_SELF == libc::IN_DELETE_SELF {
        println!("{wd} -- DELETE_SELF");
    }

    if mask & libc::IN_MOVE_SELF == libc::IN_MOVE_SELF {
        println!("{wd} -- MOVE_SELF");
    }

    if mask & libc::IN_Q_OVERFLOW == libc::IN_Q_OVERFLOW {
        println!("{wd} -- Q_OVERFLOW");
    }

    if mask & libc::IN_IGNORED == libc::IN_IGNORED {
        println!("{wd} -- IGNORED");
    }
}
*/

pub type Id = u32;

pub struct Handle {
    fd: RawFd,
    wd: RawFd,

    waker: *mut AtomicWaker,
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let ret = libc::inotify_rm_watch(self.fd, self.wd);
            if ret == 0 {
                return;
            }

            let err = io::Error::last_os_error();
            if let Some(raw) = err.raw_os_error()
                && raw == 22
            {
                // file removed, this is totally fine
                return;
            }

            panic!(
                "inotify_rm_watch failed, fd: {}, wd: {}, err: {}",
                self.fd, self.wd, err
            );
        }
    }
}

unsafe impl Send for Handle {}

impl Handle {
    #[inline]
    pub fn register(&self, waker: &Waker) {
        unsafe {
            (&*self.waker).register(waker);
        }
    }

    pub fn id(&self) -> Id {
        self.wd as u32
    }
}
