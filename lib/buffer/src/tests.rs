use std::fmt::{Display, Formatter};

use crate::config::{BufferConfig, BufferType, WhenFull};
use crate::{BufferReceiver, BufferSender, Encodable};
use bytes::{Buf, BufMut};
use finalize::{
    AddBatchNotifier, BatchNotifier, EventFinalizer, EventFinalizers, EventStatus, Finalizable,
};
use rand::Rng;
use rand::distr::Alphanumeric;
use tokio_test::task::spawn;
use tokio_test::{assert_pending, assert_ready};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug)]
pub struct MessageError;

impl Display for MessageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("message error")
    }
}

impl std::error::Error for MessageError {}

#[derive(Clone, Debug, Eq)]
pub struct Message {
    pub size: usize,
    pub decode_err: bool,
    pub finalizer: EventFinalizers,
}

impl Drop for Message {
    fn drop(&mut self) {
        let finalizers = self.finalizer.take_finalizers();
        drop(finalizers);
    }
}

impl From<usize> for Message {
    fn from(size: usize) -> Self {
        Self {
            size,
            decode_err: false,
            finalizer: EventFinalizers::default(),
        }
    }
}

impl From<(usize, bool)> for Message {
    fn from((size, decode_err): (usize, bool)) -> Self {
        Self {
            size,
            decode_err,
            finalizer: Default::default(),
        }
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size
    }
}

impl AddBatchNotifier for Message {
    fn add_batch_notifier(&mut self, notifier: BatchNotifier) {
        self.finalizer.add(EventFinalizer::new(notifier));
    }
}

impl Finalizable for Message {
    fn take_finalizers(&mut self) -> EventFinalizers {
        std::mem::take(&mut self.finalizer)
    }
}

impl Encodable for Message {
    type Error = MessageError;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), Self::Error> {
        let c = if self.decode_err { 0 } else { 1 };

        buf.put_bytes(c, self.size);

        Ok(())
    }

    fn decode<B: Buf>(mut buf: B) -> Result<Self, Self::Error> {
        let size = buf.remaining();
        let first = buf.get_u8();
        if first == 0 {
            return Err(MessageError);
        };

        Ok(Self {
            size,
            decode_err: false,
            finalizer: EventFinalizers::default(),
        })
    }

    fn byte_size(&self) -> usize {
        self.size
    }
}

impl Message {
    pub fn new(size: usize) -> Self {
        assert!(size > 4);

        Self {
            size,
            decode_err: false,
            finalizer: EventFinalizers::default(),
        }
    }

    pub async fn acknowledge(&mut self) {
        self.take_finalizers().update_status(EventStatus::Delivered);

        tokio::task::yield_now().await;
    }
}

async fn build_buffer<T: Encodable + Unpin>(
    config: BufferConfig,
) -> (BufferSender<T>, BufferReceiver<T>) {
    let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_env_filter(EnvFilter::from_default_env())
        // .with_span_events(FmtSpan::ENTER)
        .finish()
        .try_init(); // unwrap cannot be handled here, because tests might be run parallelly

    let mut rng = rand::rng();
    let id = (&mut rng)
        .sample_iter(Alphanumeric)
        .take(8)
        .map(char::from)
        .collect::<String>();

    config
        .build(id, std::env::temp_dir().join("buffer_tests"))
        .unwrap()
}

async fn run_cases<F, T>(configs: Vec<BufferConfig>, f: F)
where
    T: Encodable + Unpin,
    F: AsyncFn(BufferSender<T>, BufferReceiver<T>),
{
    for config in configs {
        let (tx, rx) = build_buffer::<T>(config).await;

        f(tx, rx).await;
    }
}

#[tokio::test]
async fn block_when_writer_hit_buffer_limit() {
    run_cases::<_, Message>(
        vec![
            BufferConfig {
                when_full: WhenFull::Block,
                typ: BufferType::Memory {
                    // memory based variant has no header, of cause, so the byte_size
                    // is the Message's size, 35 is enough for 3.5 messages
                    max_size: 35,
                },
            },
            BufferConfig {
                when_full: WhenFull::Block,
                typ: BufferType::Disk {
                    // record header length is added, so the actual record size is
                    // Message.size + 4 + 4 + 8
                    max_size: 100,
                    max_chunk_size: 1000,
                    max_record_size: 1000,
                },
            },
        ],
        async |mut tx: BufferSender<Message>, mut rx| {
            tx.send(10.into()).await.unwrap();
            tx.send(10.into()).await.unwrap();
            tx.send(10.into()).await.unwrap();
            let mut send = spawn(async { tx.send(10.into()).await.unwrap() });
            assert_pending!(send.poll());

            let mut recv1 = spawn(async { rx.next().await.unwrap() });
            assert_ready!(recv1.poll());
        },
    )
    .await;
}

#[tokio::test]
async fn disk_reader_skip_decode_error() {
    let config = BufferConfig {
        when_full: WhenFull::Block,
        typ: BufferType::Disk {
            // record header length is added, so the actual record size is
            // Message.size + 4 + 4 + 8
            max_size: 100,
            max_chunk_size: 1000,
            max_record_size: 1000,
        },
    };
    let (mut tx, mut rx) = build_buffer::<Message>(config).await;

    tx.send(10.into()).await.unwrap();
    tx.send((11, true).into()).await.unwrap();
    tx.send(12.into()).await.unwrap();

    let mut msg = rx.next().await.unwrap();
    msg.acknowledge().await;
    assert_eq!(msg.size, 10);

    let mut msg = rx.next().await.unwrap();
    msg.acknowledge().await;
    assert_eq!(msg.size, 12);
}
