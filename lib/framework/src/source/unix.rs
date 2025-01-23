use std::fs::remove_file;
use std::path::PathBuf;
use std::pin::pin;

use codecs::decoding::DecodeError;
use codecs::decoding::StreamDecodingError;
use event::Events;
use futures_util::{FutureExt, StreamExt};
use tokio::io::AsyncWriteExt;
use tokio::net::{UnixListener, UnixStream};
use tokio_stream::wrappers::UnixListenerStream;
use tokio_util::codec::{Decoder, FramedRead};

use crate::async_read::VecAsyncReadExt;
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;
use crate::Source;

/// Returns a `Source` object corresponding to a Unix domain stream socket.
/// Passing in different functions for `decoder` and `handle_events` can allow
/// for different source-specific logic (such as decoding syslog messages in the
/// syslog source).
pub fn build_unix_stream_source<D, H>(
    path: PathBuf,
    decoder: D,
    handle_events: H,
    shutdown: ShutdownSignal,
    out: Pipeline,
) -> Source
where
    D: Clone + Send + Decoder<Item = (Events, usize), Error = DecodeError> + 'static,
    H: Fn(&mut Events, usize) + Clone + Send + Sync + 'static,
{
    Box::pin(async move {
        let listener = UnixListener::bind(&path).expect("Failed to bind to listener socket");

        info!(message = "Listening", ?path, r#type = "unix");

        let stream = UnixListenerStream::new(listener).take_until(shutdown.clone());
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
            let handle_events = handle_events.clone();

            let stream = socket.allow_read_until(shutdown.clone().map(|_| ()));
            let mut stream = FramedRead::new(stream, decoder.clone());

            let mut output = out.clone();
            tokio::spawn(async move {
                loop {
                    match stream.next().await {
                        Some(Ok((mut events, byte_size))) => {
                            handle_events(&mut events, byte_size);

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
    })
}
