use criterion::{criterion_group, criterion_main, Criterion};

pub fn register(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics");

    group.bench_function("without_labels", |b| {
        b.iter(|| {
            metrics::register_counter("foo", "foo description")
                .recorder(&[])
                .inc(1);
        })
    });

    group.bench_function("with_2_labels", |b| {
        b.iter(|| {
            metrics::register_counter("counter2", "counter 2 description")
                .recorder(&[("key1", "value"), ("key2", "value")])
                .inc(1);
        })
    });

    group.bench_function("with_4_labels", |b| {
        b.iter(|| {
            metrics::register_counter("counter2", "counter 2 description")
                .recorder(&[
                    ("key1", "value"),
                    ("key2", "value"),
                    ("key3", "value"),
                    ("key4", "value"),
                ])
                .inc(1);
        })
    });
}

criterion_group!(benches, register);
criterion_main!(benches);
