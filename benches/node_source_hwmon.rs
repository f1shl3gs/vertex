use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main, measurement::WallTime};
use vertex::sources::node::hwmon::gather;

pub fn hwmon_gather(c: &mut Criterion) -> &mut Criterion<WallTime> {
    let path: PathBuf = "tests/node/sys".into();

    c.bench_function("hwmon_gather", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| gather(path.clone()))
    })
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = hwmon_gather
);
criterion_main!(benches);
