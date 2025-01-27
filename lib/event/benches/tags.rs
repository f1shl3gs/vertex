use criterion::{criterion_group, criterion_main, Criterion};
use event::tags::Tags;
use rand::seq::SliceRandom;

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("tags");
    let mut tr = rand::rng();
    let mut keys = [
        "key1", "key2", "key3", "key4", "key5", "key6", "key7", "key8", "key9", "key10", "key11",
        "key12", "key13", "key14", "key15", "key16", "key17", "key18", "key19", "key20",
    ];
    keys.shuffle(&mut tr);

    group.bench_function("insert/1", |b| b.iter(|| insert_keys(&keys, 1)));

    group.bench_function("insert/5", |b| b.iter(|| insert_keys(&keys, 5)));

    group.bench_function("insert/10", |b| b.iter(|| insert_keys(&keys, 10)));

    group.bench_function("insert/20", |b| b.iter(|| insert_keys(&keys, 20)));
}

#[allow(clippy::needless_range_loop)]
#[inline]
fn insert_keys(keys: &[&'static str], n: usize) {
    let mut tags = Tags::default();
    for i in 0..n {
        tags.insert(keys[i], i as i64);
    }
}

criterion_group!(benches, bench_insert);
criterion_main!(benches);
