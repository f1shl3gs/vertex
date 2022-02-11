use criterion::measurement::WallTime;
use criterion::{criterion_group, criterion_main, Criterion};

use humanize::{duration, parse_duration};

pub fn parse_duration_benchmark(c: &mut Criterion) -> &mut Criterion<WallTime> {
    c.bench_function("parse_duration", |b| {
        b.iter(|| {
            parse_duration("3m20s").unwrap();
        })
    });

    c.bench_function("duration_to_string", |b| {
        let d = parse_duration("1h20m30s40ms").unwrap();

        b.iter(|| duration(&d))
    })
}

criterion_group!(benches, parse_duration_benchmark);
criterion_main!(benches);
