use criterion::{
    Criterion,
    criterion_group,
    criterion_main,
    measurement::WallTime,
    BenchmarkId
};
use vertex::sources::node::hwmon::gather;

pub fn hwmon_gather(c: &mut Criterion) -> &mut Criterion<WallTime> {
    let path = "testdata/sys";

    c.bench_function("hwmon_gather", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                gather(path)
            })
    })
}

criterion_group!(benches, hwmon_gather);
criterion_main!(benches);