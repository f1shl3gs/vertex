use criterion::{criterion_group, criterion_main, measurement::WallTime, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use vertex::sources::node::hwmon::gather;

pub fn hwmon_gather(c: &mut Criterion) -> &mut Criterion<WallTime> {
    let path = "tests/fixtures/sys";

    c.bench_function("hwmon_gather", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| gather(path))
    })
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = hwmon_gather
);
criterion_main!(benches);
