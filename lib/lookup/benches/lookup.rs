use criterion::{criterion_group, criterion_main, Criterion};
use lookup::Path;

criterion_group!(
    name = benches;
    config = Criterion::default().noise_threshold(0.05);
    targets = bench_lookup
);
criterion_main!(benches);

fn bench_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("lookup");
    let lookup_str = "foo.bar.asdf[7].asdf";
    let lookup_str_escaped = "foo.\"b.ar\".\"asdf\\\"asdf\".asdf[7].asdf";

    group.bench_function("parse", |b| {
        b.iter(|| Path::segment_iter(&lookup_str).count())
    });

    group.bench_function("parse_escaped", |b| {
        b.iter(|| Path::segment_iter(&lookup_str_escaped).count())
    });
}
