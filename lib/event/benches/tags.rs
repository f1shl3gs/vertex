use criterion::{criterion_group, criterion_main, Criterion};
use event::tags::{Key, Tags};

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("tags");

    group.bench_function("insert/1", |b| b.iter(|| insert_keys(Tags::default(), 1)));

    group.bench_function("insert/5", |b| b.iter(|| insert_keys(Tags::default(), 5)));

    group.bench_function("insert/10", |b| b.iter(|| insert_keys(Tags::default(), 10)));

    group.bench_function("insert/20", |b| b.iter(|| insert_keys(Tags::default(), 20)));
}

const MAP_KEYS: [Key; 20] = [
    Key::from_static("key1"),
    Key::from_static("key2"),
    Key::from_static("key3"),
    Key::from_static("key4"),
    Key::from_static("key5"),
    Key::from_static("key6"),
    Key::from_static("key7"),
    Key::from_static("key8"),
    Key::from_static("key9"),
    Key::from_static("key10"),
    Key::from_static("key11"),
    Key::from_static("key12"),
    Key::from_static("key13"),
    Key::from_static("key14"),
    Key::from_static("key15"),
    Key::from_static("key16"),
    Key::from_static("key17"),
    Key::from_static("key18"),
    Key::from_static("key19"),
    Key::from_static("key20"),
];

fn insert_keys(mut attrs: Tags, n: usize) {
    for (i, key) in MAP_KEYS.into_iter().enumerate().take(n) {
        attrs.insert(key, i as i64)
    }
}

criterion_group!(benches, bench_insert);
criterion_main!(benches);
