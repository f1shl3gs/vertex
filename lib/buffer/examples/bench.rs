use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use buffer::{BufferConfig, BufferReceiver, BufferSender, BufferType, Encodable};
use bytes::{Buf, BufMut};
use finalize::{AddBatchNotifier, BatchNotifier};
use rand::Rng;
use rand::distr::Alphanumeric;

pub struct Message {
    size: usize,
}

impl AddBatchNotifier for Message {
    fn add_batch_notifier(&mut self, notifier: BatchNotifier) {
        drop(notifier);
    }
}

impl Debug for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Message").field("size", &self.size).finish()
    }
}

impl Encodable for Message {
    type Error = std::io::Error;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), Self::Error> {
        let len = self.size.to_be_bytes();
        buf.put_slice(&len);
        buf.put_bytes(1, self.size);

        Ok(())
    }

    fn decode<B: Buf>(mut buf: B) -> Result<Self, Self::Error> {
        let size = buf.get_u64() as usize;
        buf.advance(size);

        Ok(Self { size })
    }

    fn byte_size(&self) -> usize {
        8 + self.size
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let max_records = 100_0000;

    bench(
        max_records,
        &[128, 256, 512, 1024, 2048, 4096],
        BufferType::Memory {
            max_size: 4 * 1024 * 1024,
        },
    )
    .await;

    profile(async move || {
        bench(
            max_records,
            &[128, 256, 512, 1024, 2048, 4096 /*8192, 16384*/],
            BufferType::Disk {
                max_size: 4 * 1024 * 1024 * 1024,  // 4G
                max_record_size: 4 * 1024 * 1024,  // 4M
                max_chunk_size: 128 * 1024 * 1024, // 128M
            },
        )
        .await;
    })
    .await;
}

async fn profile(f: impl AsyncFn()) {
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    f().await;

    let report = guard.report().build().unwrap();

    let d = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let writer = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(format!("{}.svg", d.as_secs()))
        .unwrap();

    report.flamegraph(writer).unwrap()
}

async fn bench(records: usize, record_sizes: &[usize], variant: BufferType) {
    let variant_str = match &variant {
        BufferType::Memory { .. } => "Memory",
        BufferType::Disk { .. } => "Disk",
    };

    println!("-------------- {variant_str} --------------");
    println!(" RECORD_SIZE         BYTES    RECORDS_PER_SEC  BYTES_PER_SEC     TIME");
    for record_size in record_sizes {
        let elapsed = write_and_read(*record_size, records, variant.clone()).await;

        let rps = records as f64 / elapsed.as_secs_f64();
        let bps = (record_size * records) as f64 / 1024.0 / 1024.0 / elapsed.as_secs_f64();

        println!(
            "{:12}  {:>12}{:>15.2} r/s  {:>9.2} M/s{:>8.2}s",
            record_size,
            humanize::bytes::bytes(record_size * records),
            rps,
            bps,
            elapsed.as_secs_f64()
        );
    }
}

async fn write_and_read(record_size: usize, max_records: usize, variant: BufferType) -> Duration {
    let (mut tx, mut rx, path) = setup(variant);

    let start = Instant::now();
    let write_handle = tokio::spawn(async move {
        for _ in 0..max_records {
            let msg = Message { size: record_size };
            tx.send(msg).await.unwrap();
        }
    });

    for _ in 0..max_records {
        rx.next().await.unwrap();
    }

    write_handle.await.unwrap();

    let elapsed = start.elapsed();

    std::fs::remove_dir_all(path).unwrap();

    elapsed
}

fn setup(typ: BufferType) -> (BufferSender<Message>, BufferReceiver<Message>, PathBuf) {
    let mut rng = rand::rng();
    let id = (&mut rng)
        .sample_iter(Alphanumeric)
        .take(8)
        .map(char::from)
        .collect::<String>();

    let root = std::env::temp_dir();
    let path = root.join(&id);
    std::fs::create_dir_all(&path).unwrap();

    let (tx, rx) = BufferConfig {
        when_full: Default::default(),
        typ,
    }
    .build(id, root)
    .unwrap();

    (tx, rx, path)
}
