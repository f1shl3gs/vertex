use condition::parse;
use criterion::{criterion_group, criterion_main, Criterion};
use event::{fields, LogRecord};

fn bench_eval(c: &mut Criterion) {
    let mut group = c.benchmark_group("condition");
    let log = &LogRecord::from(fields!(
        "number" => "1",
        "message" => "info blah blah",
        "map" => fields!(
            "number" => 1.2
        )
    ));

    let expressions = [
        ("ordering", ".number >= 1"),
        ("contains", ".message contains info"),
        ("nested", ".map.number >= 1"),
        ("match", ".message match .*"),
        (
            "ordering_and_contains",
            ".message contains info and .number >= 1",
        ),
    ];

    for (name, expr) in expressions {
        group.bench_function(name, |b| {
            let expr = parse(expr).unwrap();

            b.iter(|| expr.eval(log).expect("eval failed"))
        });
    }
}

criterion_group!(benches, bench_eval);
criterion_main!(benches);
