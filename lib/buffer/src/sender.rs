use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::trace;

use crate::Encodable;
use crate::channel::{LimitedError, LimitedSender};
use crate::config::WhenFull;
use crate::disk::{Writer, WriterError};

#[derive(Debug, thiserror::Error)]
pub enum Error<T: Encodable> {
    #[error("channel closed already")]
    Closed(T),

    #[error("size limit exceeded")]
    LimitExceeded(T),

    #[error(transparent)]
    Write(WriterError<T>),
}

impl<T: Encodable> From<LimitedError<T>> for Error<T> {
    fn from(err: LimitedError<T>) -> Self {
        match err {
            LimitedError::Closed(item) => Error::Closed(item),
            LimitedError::LimitExceeded(item) => Error::LimitExceeded(item),
        }
    }
}

impl<T: Encodable> From<WriterError<T>> for Error<T> {
    fn from(err: WriterError<T>) -> Self {
        Error::Write(err)
    }
}

#[derive(Clone)]
enum Adapter<T: Encodable> {
    Memory(LimitedSender<T>),
    Disk(Arc<Mutex<Writer<T>>>),
}

impl<T: Encodable> Adapter<T> {
    async fn send(&self, item: T) -> Result<(), Error<T>> {
        match self {
            Self::Memory(tx) => tx.send(item).await?,
            Self::Disk(writer) => {
                let mut writer = writer.lock().await;

                let _ = writer.write(item).await?;
            }
        }

        Ok(())
    }

    async fn try_send(&self, item: T) -> Result<(), Error<T>> {
        match self {
            Self::Memory(tx) => tx.try_send(item).await?,
            Self::Disk(writer) => {
                let mut writer = writer.lock().await;

                writer.try_write(item).await?;
            }
        }

        Ok(())
    }
}

/// Adapter for papering over various sender backends by providing a [`Sink`] interface.
#[derive(Clone)]
pub struct BufferSender<T: Encodable> {
    adapter: Adapter<T>,

    when_full: WhenFull,
}

impl<T: Encodable> BufferSender<T> {
    pub(crate) fn memory(tx: LimitedSender<T>, when_full: WhenFull) -> Self {
        Self {
            adapter: Adapter::Memory(tx),
            when_full,
        }
    }

    pub(crate) fn disk(tx: Writer<T>, when_full: WhenFull) -> Self {
        Self {
            adapter: Adapter::Disk(Arc::new(Mutex::new(tx))),
            when_full,
        }
    }

    pub async fn send(&mut self, item: T) -> Result<(), Error<T>> {
        match self.when_full {
            WhenFull::Block => self.adapter.send(item).await,
            WhenFull::DropNewest => {
                let Err(err) = self.adapter.try_send(item).await else {
                    return Ok(());
                };

                match err {
                    Error::Closed(_) | Error::LimitExceeded(_) => Err(err),
                    Error::Write(err) => {
                        trace!(message = "drop newest item", ?err);
                        Ok(())
                    }
                }
            }
        }
    }

    pub async fn flush(&self) -> std::io::Result<()> {
        if let Adapter::Disk(writer) = &self.adapter {
            writer.lock().await.flush()?;
        }

        Ok(())
    }
}
