use std::fs;
use std::fs::remove_file;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::pin::pin;

use bytes::{Bytes, BytesMut};
use codecs::decoding::Decoder;
use codecs::decoding::StreamDecodingError;
use event::Events;
use futures::{FutureExt, StreamExt};
use tokio::io::AsyncWriteExt;
use tokio::net::{UnixDatagram, UnixListener, UnixStream};
use tokio_stream::wrappers::UnixListenerStream;
use tokio_util::codec::FramedRead;

use crate::Source;
use crate::async_read::VecAsyncReadExt;
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;

pub const UNNAMED_SOCKET_HOST: &str = "(unnamed)";

/// Returns a `Source` object corresponding to a Unix domain stream socket.
/// Passing in different functions for `decoder` and `handle_events` can allow
/// for different source-specific logic (such as decoding syslog messages in the
/// syslog source).
pub fn build_unix_stream_source(
    path: PathBuf,
    decoder: Decoder,
    handle_events: impl Fn(&mut Events, Option<&Bytes>) + Clone + Send + Sync + 'static,
    shutdown: ShutdownSignal,
    out: Pipeline,
) -> crate::Result<Source> {
    let listener = UnixListener::bind(&path)?;
    let stream = UnixListenerStream::new(listener).take_until(shutdown.clone());

    Ok(Box::pin(async move {
        info!(message = "Listening", ?path, r#type = "unix_stream");

        let mut stream = pin!(stream);
        while let Some(socket) = stream.next().await {
            let socket = match socket {
                Ok(socket) => socket,
                Err(err) => {
                    error!(
                        message = "Failed to accept socket",
                        %err
                    );

                    continue;
                }
            };

            let path = path.clone();

            let received_from = socket
                .peer_addr()
                .ok()
                .and_then(|addr| {
                    addr.as_pathname()
                        .map(|err| err.to_owned())
                        .map(|path| path.to_string_lossy().to_string().into())
                })
                // In most cases, we'll be connecting to this socket from
                // an unnamed socket (a socket not bound to a
                // file). Instead of a filename, we'll surface a specific
                // host value.
                .unwrap_or_else(|| UNNAMED_SOCKET_HOST.into());
            let received_from = Some(received_from);

            let handle_events = handle_events.clone();

            let stream = socket.allow_read_until(shutdown.clone().map(|_| ()));
            let mut stream = FramedRead::new(stream, decoder.clone());

            let mut output = out.clone();
            tokio::spawn(async move {
                loop {
                    match stream.next().await {
                        Some(Ok((mut events, _byte_size))) => {
                            handle_events(&mut events, received_from.as_ref());

                            if let Err(err) = output.send(events).await {
                                error!(
                                    message = "Error sending line",
                                    %err
                                );
                            }
                        }

                        Some(Err(err)) => {
                            debug!(
                                message = "Unix socket error",
                                %err,
                                path = ?path
                            );

                            if !err.can_continue() {
                                break;
                            }
                        }

                        None => break,
                    }
                }

                info!(message = "Finished sending");

                let socket: &mut UnixStream = stream.get_mut().get_mut();
                if let Err(err) = socket.shutdown().await {
                    error!(
                        message = "Failed shutting down socket",
                        %err
                    );
                }
            });
        }

        // Cleanup
        #[allow(clippy::drop_non_drop)]
        drop(stream);

        // TODO:
        // Wait for open connections to finish

        // Delete socket file
        if let Err(err) = remove_file(&path) {
            warn!(
                message = "Failed to deleting unix socket file",
                path = ?path,
                %err
            );
        }

        Ok(())
    }))
}

pub fn build_unix_datagram_source(
    path: PathBuf,
    permissions: Option<u32>,
    max_length: usize,
    decoder: Decoder,
    handle_events: impl Fn(&mut Events, Option<&Bytes>) + Clone + Send + Sync + 'static,
    shutdown: ShutdownSignal,
    output: Pipeline,
) -> crate::Result<Source> {
    let socket = UnixDatagram::bind(&path)?;

    info!(message = "Listening", ?path, r#type = "unix_datagram");

    change_socket_permissions(&path, permissions)?;

    Ok(Box::pin(async move {
        let result = serve(socket, max_length, decoder, handle_events, shutdown, output).await;

        // Delete the socket file
        if let Err(err) = remove_file(&path) {
            warn!(message = "failed in deleting unix socket file", ?err, ?path);
        }

        result
    }))
}

pub fn change_socket_permissions(path: &Path, permissions: Option<u32>) -> std::io::Result<()> {
    if let Some(perm) = permissions {
        return match fs::set_permissions(path, fs::Permissions::from_mode(perm)) {
            Ok(_) => Ok(()),
            Err(set_err) => {
                if let Err(err) = remove_file(path) {
                    error!(
                        message = "Failed in deleting unix socket file",
                        ?path,
                        %err,
                    );
                }

                Err(set_err)
            }
        };
    }

    Ok(())
}

async fn serve(
    socket: UnixDatagram,
    max_length: usize,
    decoder: Decoder,
    handle_events: impl Fn(&mut Events, Option<&Bytes>) + Clone + Send + Sync + 'static,
    mut shutdown: ShutdownSignal,
    mut output: Pipeline,
) -> Result<(), ()> {
    let mut buf = BytesMut::with_capacity(max_length);

    loop {
        buf.resize(max_length, 0);

        tokio::select! {
            recv = socket.recv_from(&mut buf) => {
                let (size, addr) = recv.map_err(|err| {
                    error!(
                        message = "Error receiving data from Unix Socket",
                        %err,
                        socket = "unix",
                        internal_log_rate_limit = true
                    );
                })?;

                let received_from = if !addr.is_unnamed() {
                    let path = addr.as_pathname().map(|err| err.to_owned());

                    path.map(|p| p.to_string_lossy().into_owned().into())
                } else {
                    // In most cases, we'll be connecting to this socket from an
                    // unnamed socket (a socket not bound to a file). Instead of a
                    // filename, we'll surface a specific host value.
                    Some(UNNAMED_SOCKET_HOST.into())
                };

                let payload = buf.split_to(size);

                let mut stream = FramedRead::new(payload.as_ref(), decoder.clone());
                while let Some(result) = stream.next().await {
                    match result {
                        Ok((mut events, _byte_size)) => {
                            handle_events(&mut events, received_from.as_ref());

                            if let Err(_err) = output.send(events).await {
                                warn!(
                                    message = "failed to forward events, cause downstream is closed"
                                );

                                break
                            }
                        }
                        Err(err) => {
                            warn!(
                                message = "Error while receiving data from Unix socket",
                                internal_log_rate_limit = true,
                            );

                            if !err.can_continue() {
                                break
                            }
                        }
                    }
                }
            },
            _ = &mut shutdown => break,
        }
    }

    Ok(())
}
