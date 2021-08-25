use criterion::{criterion_group, criterion_main, Criterion};

use vertex::duration::parse_duration;
use criterion::measurement::WallTime;

pub fn parse_duration_benchmark(c: &mut Criterion) -> &mut Criterion<WallTime> {
    c.bench_function("parse_duration", |b| {
        b.iter(|| {
            parse_duration("3m20s").unwrap();
        })
    })
}

criterion_group!(benches, parse_duration_benchmark);
criterion_main!(benches);