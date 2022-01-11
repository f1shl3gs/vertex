use crate::async_read::VecAsyncReadExt;
use crate::codecs;
use crate::codecs::StreamDecodingError;
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;
use crate::sources::Source;
use bytes::Bytes;
use event::Event;
use futures::stream;
use futures_util::{FutureExt, StreamExt};
use std::fs::remove_file;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::net::{UnixListener, UnixStream};
use tokio_stream::wrappers::UnixListenerStream;
use tokio_util::codec::FramedRead;

/// Returns a `Source` object corresponding to a Unix domain stream socket.
/// Passing in different functions for `decoder` and `handle_events` can allow
/// for different source-specific logic (such as decoding syslog messages in the
/// syslog source).
pub fn build_unix_stream_source(
    path: PathBuf,
    decoder: codecs::Decoder,
    handle_events: impl Fn(&mut [Event], Option<Bytes>, usize) + Clone + Send + Sync + 'static,
    shutdown: ShutdownSignal,
    out: Pipeline,
) -> Source {
    Box::pin(async move {
        let listener = UnixListener::bind(&path).expect("Failed to bind to listener socket");

        info!(message = "Listening", ?path, r#type = "unix");

        let stream = UnixListenerStream::new(listener).take_until(shutdown.clone());
        tokio::pin!(stream);

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
            let np = if let Ok(addr) = socket.peer_addr() {
                if let Some(p) = addr.as_pathname().map(|err| err.to_owned()) {
                    Some(p)
                } else {
                    None
                }
            } else {
                None
            };

            let handle_events = handle_events.clone();
            let received_from: Option<Bytes> = np.map(|p| p.to_string_lossy().into_owned().into());

            let stream = socket.allow_read_until(shutdown.clone().map(|_| ()));
            let mut stream = FramedRead::new(stream, decoder.clone());

            let mut output = out.clone();
            tokio::spawn(async move {
                loop {
                    match stream.next().await {
                        Some(Ok((mut events, byte_size))) => {
                            handle_events(&mut events, received_from.clone(), byte_size);

                            if let Err(err) = output.send_all(stream::iter(events)).await {
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

                info!("Finished sending");

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
    })
}
