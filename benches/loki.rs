use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use framework::template::Template;
use vertex::sinks::loki::valid_label_name;

const VALID: [&str; 4] = ["name", " name ", "bee_bop", "a09b"];
const INVALID: [&str; 4] = ["0ab", "*", "", " "];

fn bench_valid_label_name(c: &mut Criterion) {
    let mut group = c.benchmark_group("loki");

    group.throughput(Throughput::Elements(1));
    group.bench_function("valid_label_name", |b| {
        for tmpl in VALID {
            b.iter_batched(
                || Template::try_from(tmpl).unwrap(),
                |label| valid_label_name(&label),
                BatchSize::SmallInput,
            );
        }

        for tmpl in INVALID {
            b.iter_batched(
                || Template::try_from(tmpl).unwrap(),
                |label| valid_label_name(&label),
                BatchSize::SmallInput,
            );
        }
    });
}

criterion_group!(benches, bench_valid_label_name);
criterion_main!(benches);
