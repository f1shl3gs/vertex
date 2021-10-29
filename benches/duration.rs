use criterion::{criterion_group, criterion_main, Criterion};
use criterion::measurement::WallTime;

use vertex::duration::{parse_duration, duration_to_string};

pub fn parse_duration_benchmark(c: &mut Criterion) -> &mut Criterion<WallTime> {
    c.bench_function("parse_duration", |b| {
        b.iter(|| {
            parse_duration("3m20s").unwrap();
        })
    });

    c.bench_function("duration_to_string", |b| {
        let d = parse_duration("1h20m30s40ms").unwrap();

        b.iter(|| {
            duration_to_string(&d)
        })
    })
}

criterion_group!(benches, parse_duration_benchmark);
criterion_main!(benches);