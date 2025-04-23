use futures::StreamExt;
use futures::stream::BoxStream;
use tracing::error;

use crate::Encodable;
use crate::channel::LimitedReceiver;
use crate::disk::{Reader, ReaderError};

pub enum BufferReceiver<T: Encodable + Unpin> {
    Memory(LimitedReceiver<T>),

    Disk(Reader<T>),
}

impl<T: Encodable + Unpin + Sync> BufferReceiver<T> {
    pub async fn next(&mut self) -> Option<T> {
        match self {
            BufferReceiver::Memory(receiver) => receiver.next().await,
            BufferReceiver::Disk(reader) => loop {
                match reader.read().await {
                    Ok(result) => break result,
                    Err(err) => match err {
                        ReaderError::Io(err) => {
                            panic!("Reader encountered unrecoverable error, {err}")
                        }
                        ReaderError::Checksum { .. }
                        | ReaderError::Decode { .. }
                        | ReaderError::PartialWrite => {
                            error!(
                                message = "Error encountered during buffer read",
                                ?err,
                                internal_log_rate_limit = true,
                            );

                            continue;
                        }
                    },
                }
            },
        }
    }

    pub fn into_stream(self) -> BoxStream<'static, T> {
        futures::stream::unfold(self, |mut receiver| async {
            let item = receiver.next().await?;

            Some((item, receiver))
        })
        .boxed()
    }
}
