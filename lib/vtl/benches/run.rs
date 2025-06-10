use criterion::{Criterion, criterion_group, criterion_main};
use value::{Value, value};
use vtl::{Context, TargetValue};

const SCRIPT: &str = include_str!("example.vtl");

fn run(c: &mut Criterion) {
    let program = vtl::compile(SCRIPT).unwrap();
    let mut target = TargetValue {
        metadata: value!({
            "partition": 1,
            "offset": 123,
        }),
        value: value!({
            "msg": "{\"foo\": \"bar\"}",
            "index": 5,
            "array": [1, 2, 3, {"ak": "av"}],
            "map": {"k1": "k2"},
        }),
    };
    let mut variables = vec![Value::Null; program.type_state().variables.len()];
    let mut cx = Context {
        target: &mut target,
        variables: &mut variables,
    };

    c.bench_function("run", |b| {
        b.iter(|| {
            program.resolve(&mut cx).unwrap();
        })
    });
}

criterion_group!(benches, run);
criterion_main!(benches);
