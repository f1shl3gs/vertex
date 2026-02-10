use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use humanize::bytes::parse_bytes;
use rand::RngExt;
use rand::distr::Alphanumeric;

static LINES: AtomicU64 = AtomicU64::new(0);

fn rand_string(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect::<String>()
}

fn nginx() -> String {
    let count = LINES.fetch_add(1, Ordering::Relaxed);

    format!(
        r#"111.49.69.172 - [111.49.69.172] - - [18/Jun/2019:15:48:47 +0800] "GET /{} HTTP/1.1" 308 171 "-" "{}" {} 0.000 [default-http-svc3-80] - - - - {} {}
"#,
        rand_string(16),
        rand_string(16),
        count,
        rand_string(16),
        rand_string(100),
    )
}

fn main() {
    let mut args = std::env::args().skip(1);

    let Some(path) = args.next() else {
        println!("Usage: generate <path> <size>");
        return;
    };

    let size = match args.next() {
        Some(value) => parse_bytes(&value).unwrap(),
        None => {
            println!("Usage: generate <path> <size>");
            return;
        }
    };

    let start = Instant::now();
    let mut length = 0;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .unwrap();
    while length < size {
        let line = nginx();
        let data = line.as_bytes();
        file.write_all(data).unwrap();

        length += data.len();
    }
    let elapsed = start.elapsed();

    println!(
        "generate {} lines, in {:?}, rate: {} M/s",
        LINES.load(Ordering::Acquire),
        elapsed,
        length as f64 / 1024.0 / 1024.0 / elapsed.as_secs_f64()
    );
}
