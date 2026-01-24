use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use configurable::Configurable;
use framework::Source;
use framework::config::{Resource, SourceContext};
use serde::{Deserialize, Serialize};
use tokio::net::UnixListener;

use super::serve_conn;

#[derive(Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Absolute path to the socket file to read DNSTAP data from
    ///
    /// The DNS server must be configured to send its DNSTAP data to this
    /// socket file. The socket file is created if it doesn't already exist
    /// when the source first starts.
    path: PathBuf,

    /// Unix file mode bits to be applied to the unix socket file as its
    /// designated file permissions.
    ///
    /// Note: The file mode value can be specified in any numeric format supported
    /// by your configuration language, but it is most intuitive to use an octal number
    permission: Option<u32>,

    /// The size, in bytes, of the receive buffer used for the socket.
    ///
    /// This should not typically need to be changed.
    receive_buffer_size: Option<usize>,
}

impl Config {
    pub fn resource(&self) -> Resource {
        Resource::UnixSocket(self.path.clone())
    }

    pub async fn build(&self, max_frame_length: usize, cx: SourceContext) -> crate::Result<Source> {
        match std::fs::exists(&self.path) {
            Ok(exists) => {
                if exists {
                    std::fs::remove_file(&self.path)?;
                }
            }
            Err(err) => {
                error!(
                    message = "unable to get socket information",
                    ?err,
                    path = ?self.path
                );
                return Err(err.into());
            }
        }

        let listener = UnixListener::bind(&self.path)?;

        // the permissions to unix socket are restricted from 0o700 to 0o777,
        // which are 448 and 511 in decimal
        if let Some(permission) = self.permission {
            if !(448..=511).contains(&permission) {
                return Err(format!(
                    "invalid socket permission {permission:#o}, which must between 0o700 and 0o777",
                )
                .into());
            }

            match std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(permission))
            {
                Ok(_) => {
                    info!(
                        message = "socket permission updated",
                        path = ?self.path,
                        permission = format!("{:#o}", permission)
                    )
                }
                Err(err) => {
                    error!(
                        message = "failed to update socket permissions",
                        ?err,
                        path = ?self.path
                    );

                    return Err(err.into());
                }
            }
        }

        // system's 'net.core.rmem_max' might be changed if socket receive buffer is not updated properly
        if let Some(size) = self.receive_buffer_size {
            let ret = unsafe {
                let size = size as libc::c_int;
                let ptr = std::ptr::addr_of!(size).cast();

                libc::setsockopt(
                    listener.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_RCVBUF,
                    ptr,
                    size_of::<libc::c_int>() as libc::socklen_t,
                )
            };

            if ret == -1 {
                warn!(
                    message = "failed to set Unix socket receive buffer size",
                    ?size,
                    err = ?std::io::Error::last_os_error()
                );
            }
        }

        let mut shutdown = cx.shutdown;
        let output = cx.output;

        Ok(Box::pin(async move {
            loop {
                let stream = tokio::select! {
                    result = listener.accept() => match result {
                        Ok((stream, peer)) => {
                            debug!(
                                message = "connected",
                                ?peer
                            );

                            stream
                        },
                        Err(err) => {
                            warn!(
                                message = "accepting new connection failed",
                                ?err,
                            );

                            break
                        }
                    },
                    _ = &mut shutdown => break,
                };

                tokio::spawn(serve_conn(
                    stream,
                    true,
                    max_frame_length,
                    shutdown.clone(),
                    output.clone(),
                ));
            }

            Ok(())
        }))
    }
}
