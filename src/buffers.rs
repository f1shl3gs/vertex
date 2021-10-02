use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{
    channel::mpsc,
    Sink,
    SinkExt,
    Stream,
};
use pin_project::pin_project;
use serde::{Deserialize, Serialize};

pub use buffers::{Acker, DecodeBytes, EncodeBytes};
use crate::event::Event;


#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WhenFull {
    Block,
    DropNewest,
}

impl Default for WhenFull {
    fn default() -> Self {
        WhenFull::Block
    }
}

#[pin_project]
pub struct DropWhenFull<S> {
    #[pin]
    inner: S,
    drop: bool,
}

impl<S> DropWhenFull<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            drop: false,
        }
    }
}

impl<T, S: Sink<T> + Unpin> futures::Sink<T> for DropWhenFull<S> {
    type Error = S::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        match this.inner.poll_ready(cx) {
            Poll::Ready(Ok(())) => {
                *this.drop = false;
                Poll::Ready(Ok(()))
            }
            Poll::Pending => {
                *this.drop = true;
                Poll::Ready(Ok(()))
            }
            error @ std::task::Poll::Ready(..) => error,
        }
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        if self.drop {
            debug!("dropping events");
            Ok(())
        } else {
            self.project().inner.start_send(item)
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_close(cx)
    }
}

#[derive(Clone)]
pub enum BufferInputCloner<T>
    where
        T: Send + Sync + Unpin + Clone + EncodeBytes<T> + DecodeBytes<T>,
        <T as EncodeBytes<T>>::Error: Debug,
        <T as DecodeBytes<T>>::Error: Debug,
{
    Memory(mpsc::Sender<T>, WhenFull),
    #[cfg(feature = "disk-buffer")]
    Disk(),
}

impl<'a, T> BufferInputCloner<T>
    where
        T: 'a + Send + Sync + Unpin + Clone + EncodeBytes<T> + DecodeBytes<T>,
        <T as EncodeBytes<T>>::Error: Debug,
        <T as DecodeBytes<T>>::Error: Debug + Display,
{
    pub fn get(&self) -> Box<dyn Sink<T, Error=()> + 'a + Send + Unpin> {
        match self {
            BufferInputCloner::Memory(tx, when_full) => {
                let inner = tx.clone()
                    .sink_map_err(|err| {});

                if when_full == &WhenFull::DropNewest {
                    Box::new(DropWhenFull::new(inner))
                } else {
                    Box::new(inner)
                }
            }
            #[cfg(feature = "disk-buffer")]
            BufferInputCloner::Disk() => {
                todo!()
            }
        }
    }
}

#[derive(Deserialize, Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BufferConfig {
    Memory {
        #[serde(default = "BufferConfig::memory_max_events")]
        max_events: usize,
        #[serde(default)]
        when_full: WhenFull,
    },
    #[cfg(feature = "disk-buffer")]
    Disk {
        max_size: usize,
        #[serde(default)]
        when_full: WhenFull,
    },
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self::Memory {
            max_events: BufferConfig::memory_max_events(),
            when_full: Default::default(),
        }
    }
}

pub(crate) type EventStream = Box<dyn Stream<Item=Event> + Unpin + Send>;

impl BufferConfig {
    #[inline]
    const fn memory_max_events() -> usize {
        512
    }

    pub fn build(
        &self,
        data_dir: &PathBuf,
        name: &str,
    ) -> Result<(BufferInputCloner<Event>, EventStream, Acker), String> {
        match self {
            BufferConfig::Memory {
                max_events,
                when_full,
            } => {
                let (tx, rx) = mpsc::channel(*max_events);
                let tx = BufferInputCloner::Memory(tx, *when_full);
                let rx = Box::new(rx);
                Ok((tx, rx, Acker::Null))
            }
            #[cfg(feature = "disk-buffer")]
            BufferConfig::Disk { .. } => {
                todo!()
            }
        }
    }

    // pub fn resources(&self, name: &str) -> Vec<Resource> {
    //     match self {
    //         BufferConfig::Memory { .. } => Vec::new(),
//
    //         #[cfg(feature = "disk-buffer")]
    //         BufferConfig::Disk { .. } => vec![Resource::DiskBuffer(name.to_string())]
    //     }
    // }
}


#[cfg(test)]
mod tests {
    use futures::{channel::mpsc, Sink, Stream};

    use super::*;

    #[test]
    fn config_default_values() {
        fn check(source: &str, config: BufferConfig) {
            let conf: BufferConfig = serde_yaml::from_str(source).unwrap();
            assert_eq!(
                serde_yaml::to_string(&conf).unwrap(),
                serde_yaml::to_string(&config).unwrap(),
            )
        }

        check(
            r#"
memory: {}
        "#,
            BufferConfig::Memory {
                max_events: 512,
                when_full: WhenFull::Block,
            },
        );

        check(r#"
        memory:
            max_events: 100
        "#,
              BufferConfig::Memory {
                  max_events: 100,
                  when_full: WhenFull::Block,
              },
        );

        check(r#"
        memory:
            when_full: drop_newest
        "#,
              BufferConfig::Memory {
                  max_events: 512,
                  when_full: WhenFull::DropNewest,
              },
        )
    }

    #[tokio::test]
    async fn drop_when_full() {
        futures::future::lazy(|cx| {
            let (tx, rx) = mpsc::channel(2);
            let mut tx = Box::pin(DropWhenFull::new(tx));

            assert_eq!(tx.as_mut().poll_ready(cx), Poll::Ready(Ok(())));
            assert_eq!(tx.as_mut().start_send(1), Ok(()));
            assert_eq!(tx.as_mut().poll_ready(cx), Poll::Ready(Ok(())));
            assert_eq!(tx.as_mut().start_send(2), Ok(()));
            assert_eq!(tx.as_mut().poll_ready(cx), Poll::Ready(Ok(())));
            assert_eq!(tx.as_mut().start_send(3), Ok(()));
            assert_eq!(tx.as_mut().poll_ready(cx), Poll::Ready(Ok(())));
            assert_eq!(tx.as_mut().start_send(4), Ok(()));

            let mut rx = Box::pin(rx);

            assert_eq!(rx.as_mut().poll_next(cx), Poll::Ready(Some(1)));
            assert_eq!(rx.as_mut().poll_next(cx), Poll::Ready(Some(2)));
            assert_eq!(rx.as_mut().poll_next(cx), Poll::Ready(Some(3)));
            assert_eq!(rx.as_mut().poll_next(cx), Poll::Pending);
        }).await;
    }
}