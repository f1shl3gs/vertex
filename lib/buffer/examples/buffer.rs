use std::time::{Duration, Instant};

use tokio::task::JoinSet;

const MESSAGES: usize = 100_000_000;
const THREADS: usize = 8;

mod message {
    use std::fmt;
    use std::fmt::Display;

    use buffer::Encodable;
    use bytes::{Buf, BufMut};
    use finalize::{AddBatchNotifier, BatchNotifier};

    const LEN: usize = 1;

    #[derive(Clone, Copy)]
    pub(crate) struct Message(#[allow(dead_code)] [usize; LEN]);

    impl AddBatchNotifier for Message {
        fn add_batch_notifier(&mut self, _notifier: BatchNotifier) {
            todo!()
        }
    }

    #[derive(Debug)]
    pub enum Error {}
    impl Display for Error {
        fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
            todo!()
        }
    }

    impl std::error::Error for Error {}

    impl Encodable for Message {
        type Error = Error;

        fn encode<B: BufMut>(&self, _buf: &mut B) -> Result<(), Self::Error> {
            todo!()
        }

        fn decode<B: Buf>(_buf: B) -> Result<Self, Self::Error> {
            todo!()
        }

        fn byte_size(&self) -> usize {
            self.0.len()
        }
    }

    impl fmt::Debug for Message {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.pad("Message")
        }
    }

    #[inline]
    pub(crate) fn new(num: usize) -> Message {
        Message([num; LEN])
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 32)]
async fn main() {
    let (tx, mut rx) = buffer::limited(u32::MAX as usize);
    let mut tasks = JoinSet::new();

    let sender = tx.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(1));

        loop {
            ticker.tick().await;

            println!(
                "buffered bytes {}",
                u32::MAX as usize - sender.available_bytes()
            );
        }
    });

    let start = Instant::now();
    for tid in 0..THREADS {
        let tx = tx.clone();
        tasks.spawn(async move {
            for i in 0..MESSAGES / THREADS {
                tx.send(message::new(i)).await.unwrap();
            }

            println!("thread {tid} finished");
        });
    }

    for _ in 0..MESSAGES {
        rx.recv().await.unwrap();
    }

    tasks.join_all().await;

    let elapsed = start.elapsed();
    println!("{:7.3} sec", elapsed.as_secs_f64());
}
