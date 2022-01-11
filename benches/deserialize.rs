use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use jsmn::Token;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const INPUT: &str = r#"{
    "nesting": { "inner object": {} },
    "an array": [1.5, 1e-6],
    "string with escaped double quotes" : "\"quick brown foxes\"",
    "bool": true,
    "null": null
}"#;

fn bench_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize");

    group.throughput(Throughput::Bytes(INPUT.len() as u64));
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("jsmn", |b| {
        b.iter(|| {
            let mut parser = jsmn::JsonParser::new();
            let mut tokens = [Token::default(); 32];

            parser.parse(INPUT.as_ref(), &mut tokens).unwrap()
        });
    });

    group.bench_function("jsmn_reuse_parser", |b| {
        let mut parser = jsmn::JsonParser::new();

        b.iter(|| {
            parser.reset();
            let mut tokens = [Token::default(); 32];

            parser.parse(INPUT.as_ref(), &mut tokens).unwrap()
        });
    });

    group.bench_function("serde", |b| {
        #[derive(Debug, Deserialize, Serialize)]
        struct Empty {}

        #[derive(Debug, Deserialize, Serialize)]
        struct Nested {
            #[serde(rename = "inner object")]
            inner: Empty,
        }

        #[derive(Debug, Deserialize, Serialize)]
        struct Object {
            nesting: Nested,
            #[serde(rename = "an array")]
            array: Vec<f64>,
            #[serde(rename = "string with escaped double quotes")]
            string: String,
            bool: bool,
            null: Option<i32>,
        }

        b.iter(|| {
            let _obj: Object = serde_json::from_str(INPUT).unwrap();
        });
    });
}

criterion_group!(benches, bench_deserialize);
criterion_main!(benches);
