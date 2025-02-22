use criterion::{Criterion, Throughput, criterion_group, criterion_main};

fn bench_parse_text(c: &mut Criterion) {
    let data = std::fs::read_to_string("fixtures/node_exporter.txt").unwrap();

    let mut group = c.benchmark_group("prometheus");
    group.measurement_time(std::time::Duration::from_secs(10));
    group.noise_threshold(0.03);

    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_with_input("parse_text", data.as_str(), |b, input| {
        b.iter(|| {
            prometheus::parse_text(input).unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, bench_parse_text);
criterion_main!(benches);
