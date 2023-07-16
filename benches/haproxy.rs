use std::time::Duration;

use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion, Throughput,
};

pub fn parse_haproxy_csv(c: &mut Criterion) {
    let input = include_str!("../tests/haproxy/stats.csv");

    let mut group: BenchmarkGroup<WallTime> = c.benchmark_group("haproxy");

    group.throughput(Throughput::Bytes(input.len() as u64));
    group.measurement_time(Duration::from_secs(10));
    group.bench_function("parse_csv", |b| {
        b.iter(|| {
            let reader = std::io::Cursor::new(input);
            vertex::sources::haproxy::parse_csv(reader).unwrap();
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(120))
        // degree of noise to ignore in measurements, here 1%
        .noise_threshold(0.01)
        // likelihood of noise registering as difference, here 5%
        .significance_level(0.05)
        // likelihood of capturing the true runtime, here 95%
        .confidence_level(0.95)
        // total number of bootstrap resamples, higher is less noisy but slower
        .nresamples(100_100)
        // total samples to collect within the set measurement time
        .sample_size(150);
    targets = parse_haproxy_csv
);

criterion_main!(benches);
