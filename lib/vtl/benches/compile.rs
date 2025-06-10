use criterion::{Criterion, criterion_group, criterion_main};

const SCRIPT: &str = include_str!("example.vtl");

fn compile(c: &mut Criterion) {
    c.bench_function("compile", |b| {
        b.iter(|| {
            vtl::compile(SCRIPT).unwrap();
        })
    });
}

criterion_group!(benches, compile);
criterion_main!(benches);
