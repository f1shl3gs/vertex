use std::path::PathBuf;
use std::time::Duration;

use futures_util::StreamExt;

use crate::Error;

mod inotify {
    use std::ffi::{CString, OsStr, c_void};
    use std::io;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;
    use std::pin::Pin;
    use std::task::{Context, Poll, ready};

    use futures::Stream;
    use tokio::io::unix::AsyncFd;

    // the size of inotify_event
    const EVENT_SIZE: usize = 16;

    pub struct Watcher {
        fd: AsyncFd<OwnedFd>,
        wds: Vec<OwnedFd>,
    }

    impl Watcher {
        pub fn new() -> io::Result<Watcher> {
            let fd = unsafe {
                let ret = libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK);
                if ret == -1 {
                    return Err(io::Error::last_os_error());
                }

                OwnedFd::from_raw_fd(ret)
            };

            Ok(Watcher {
                fd: AsyncFd::new(fd)?,
                wds: vec![],
            })
        }

        pub fn add(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
            let path = CString::new(path.as_ref().as_os_str().as_bytes())?;

            let wd = unsafe {
                let ret = libc::inotify_add_watch(
                    self.fd.as_raw_fd(),
                    path.as_ptr() as *const _,
                    libc::IN_CLOSE_WRITE | libc::IN_MOVE | libc::IN_MOVED_TO | libc::IN_CREATE,
                );
                if ret == -1 {
                    return Err(io::Error::last_os_error());
                }

                OwnedFd::from_raw_fd(ret)
            };

            self.wds.push(wd);

            Ok(())
        }

        pub fn into_stream(self, buf: &[u8]) -> EventStream {
            EventStream {
                fd: self.fd,
                wds: self.wds,
                buf,
            }
        }
    }

    #[allow(dead_code)]
    pub struct EventStream<'a> {
        fd: AsyncFd<OwnedFd>,
        wds: Vec<OwnedFd>,
        buf: &'a [u8],
    }

    impl<'a> Stream for EventStream<'a> {
        type Item = io::Result<Events<'a>>;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            loop {
                let mut guard = ready!(self.fd.poll_read_ready(cx))?;

                #[allow(clippy::blocks_in_conditions)]
                match guard.try_io(|inner| {
                    let ret = unsafe {
                        libc::read(
                            inner.as_raw_fd(),
                            self.buf.as_ptr() as *mut c_void,
                            self.buf.len(),
                        )
                    };
                    if ret == -1 {
                        return Err(io::Error::last_os_error());
                    }

                    Ok(ret as usize)
                }) {
                    Ok(Ok(len)) => {
                        return Poll::Ready(Some(Ok(Events {
                            buf: self.buf,
                            pos: 0,
                            len,
                        })));
                    }
                    Ok(Err(err)) => return Poll::Ready(Some(Err(err))),
                    Err(_would_block) => continue,
                }
            }
        }
    }

    /// An inotify event
    ///
    /// A file system event that describes a change that the user previously
    /// registered interest in. To watch for events.
    #[allow(dead_code)]
    pub struct Event<S> {
        /// Identifies the watch this event originates from.
        pub wd: i32,

        /// Indicates what kind of event this is
        pub mask: u32,

        /// Connects related events to each other
        ///
        /// When a file is renamed, this results two events: [`MOVED_FROM`] and
        /// [`MOVED_TO`]. The `cookie` field will be the same for both of them,
        /// thereby making is possible to connect the event pair.
        pub cookie: u32,

        /// The name of the file the event originates from
        ///
        /// This field is set only if the subject of the event is a file or directory
        /// in watched directory. If the event concerns a file or directory that is
        /// watched directly, `name` will be `None`.
        pub name: Option<S>,
    }

    /// Iterator over inotify events.
    #[derive(Debug)]
    pub struct Events<'a> {
        buf: &'a [u8],
        pos: usize,
        len: usize,
    }

    impl<'a> Iterator for Events<'a> {
        type Item = Event<&'a OsStr>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.pos < self.len {
                let ev = unsafe {
                    let ptr = self.buf.as_ptr().add(self.pos) as *const libc::inotify_event;
                    ptr.read_unaligned()
                };

                let name = if ev.len == 0 {
                    None
                } else {
                    let name =
                        &self.buf[self.pos + EVENT_SIZE..self.pos + EVENT_SIZE + ev.len as usize];
                    let name = name.splitn(2, |b| b == &0u8).next().unwrap();

                    Some(OsStr::from_bytes(name))
                };

                self.pos += EVENT_SIZE + ev.len as usize;

                Some(Event {
                    wd: ev.wd,
                    mask: ev.mask,
                    cookie: ev.cookie,
                    name,
                })
            } else {
                None
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use futures_util::StreamExt;
        use std::fs::File;
        use std::io::Write;

        use testify::temp_dir;

        #[tokio::test]
        async fn write() {
            let directory = temp_dir();
            let filepath = directory.join("test.txt");
            let mut file = File::create(&filepath).unwrap();

            let mut watcher = Watcher::new().unwrap();
            watcher.add(directory).unwrap();
            let buf = [0u8; 4096];
            let mut stream = watcher.into_stream(&buf);

            file.write_all(&[0]).unwrap();
            file.sync_all().unwrap();
            drop(file);

            if let Some(Ok(_evs)) = stream.next().await {
                // ok
            } else {
                panic!("change should detected");
            }
        }
    }
}

/// Per notify own documentation, it's advised to have delay of more than 30 sec,
/// so to avoid receiving repetitions of previous events on MacOS.
///
/// But, config and topology reload logic can handle:
///  - Invalid config, caused either by user or by data race.
///  - Frequent changes, caused by user/editor modifying/saving file in small chunk.
///    so we can use smaller, more responsive delay.
const DEFAULT_WATCH_DELAY: Duration = Duration::from_secs(1);

const RETRY_TIMEOUT: Duration = Duration::from_secs(10);

pub fn watch_configs<'a>(
    config_paths: impl IntoIterator<Item = &'a PathBuf> + 'a,
) -> Result<(), Error> {
    let config_paths: Vec<_> = config_paths.into_iter().cloned().collect();

    // first init
    let mut watcher = inotify::Watcher::new()?;
    config_paths.iter().try_for_each(|path| watcher.add(path))?;

    info!(message = "Watching configuration files");
    tokio::spawn(async move {
        let buf = [0u8; 4096];
        let mut stream = watcher.into_stream(&buf);

        loop {
            match stream.next().await {
                Some(res) => match res {
                    Ok(_events) => {
                        info!(message = "Configuration file changed.");
                        raise_sighup();
                    }
                    Err(err) => {
                        error!(message = "read inotify failed, retrying watch", %err);

                        // error occurs, sleep a while and retry
                        tokio::time::sleep(RETRY_TIMEOUT).await;

                        drop(stream);

                        let mut watcher = inotify::Watcher::new()?;
                        config_paths.iter().try_for_each(|path| watcher.add(path))?;
                        stream = watcher.into_stream(&buf);

                        continue;
                    }
                },
                None => {
                    // this shall never happen
                    return Ok::<(), Error>(());
                }
            }

            tokio::time::sleep(DEFAULT_WATCH_DELAY).await;
        }
    });

    Ok(())
}

fn raise_sighup() {
    let ret = unsafe { libc::raise(libc::SIGHUP) };
    if ret == -1 {
        let err = std::io::Error::last_os_error();
        error!(
            message = "Unable to reload configuration file. Restart Vertex to reload it",
            %err
        );
    }
}

#[cfg(all(test, unix, not(target_os = "macos")))]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use testify::{temp_dir, temp_file};
    use tokio::signal::unix::{SignalKind, signal};

    use super::*;

    async fn test(mut file: File, timeout: Duration) -> bool {
        let mut signal = signal(SignalKind::hangup()).expect("Signal handlers should not panic");

        file.write_all(&[0]).unwrap();
        file.sync_all().unwrap();
        drop(file);

        tokio::time::timeout(timeout, signal.recv()).await.is_ok()
    }

    #[tokio::test]
    async fn file_directory_update() {
        let delay = Duration::from_secs(2);
        let directory = temp_dir();
        let filepath = directory.join("test.txt");
        let file = File::create(&filepath).unwrap();

        watch_configs(&[directory]).unwrap();

        assert!(test(file, delay * 5).await)
    }

    #[tokio::test]
    async fn file_update() {
        let delay = Duration::from_secs(3);
        let filepath = temp_file();
        let file = File::create(&filepath).unwrap();

        watch_configs(&[filepath]).unwrap();

        assert!(test(file, delay * 5).await)
    }

    #[tokio::test]
    async fn sym_file_update() {
        let delay = Duration::from_secs(3);
        let filepath = temp_file();
        let sym_file = temp_file();
        let file = File::create(&filepath).unwrap();
        std::os::unix::fs::symlink(&filepath, &sym_file).unwrap();

        watch_configs(&[filepath]).unwrap();

        assert!(test(file, delay * 5).await);
    }
}
