use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main, measurement::WallTime};
use vertex::sources::node::Paths;
use vertex::sources::node::hwmon::collect;

pub fn hwmon_gather(c: &mut Criterion) -> &mut Criterion<WallTime> {
    let paths = Paths::new(
        PathBuf::from("tests/node/fixtures"),
        PathBuf::from("tests/node/fixtures/proc"),
        PathBuf::from("tests/node/fixtures/sys"),
        PathBuf::from("tests/node/fixtures/udev"),
    );

    c.bench_function("hwmon_gather", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| collect(paths.clone()))
    })
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = hwmon_gather
);
criterion_main!(benches);
