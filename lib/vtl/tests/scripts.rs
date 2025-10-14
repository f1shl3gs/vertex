use std::time::Instant;

use value::{Value, value};
use vtl::{Diagnostic, TargetValue, compile};

#[allow(clippy::print_stdout)]
#[test]
fn run_scripts() {
    let paths = glob::glob("tests/**/*.vtl").unwrap();
    for path in paths.flatten() {
        let content = std::fs::read_to_string(&path).unwrap();

        let program = match compile(&content) {
            Ok(program) => program,
            Err(err) => {
                let output = Diagnostic::new(&content).snippets(err);
                panic!("{}", output);
            }
        };

        let mut target = TargetValue {
            metadata: Value::Object(Default::default()),
            value: value!({
                "float": 1.0,
                "int": 2,
                "str": "sss",
                "bool": true,
                "null": null,
                "array": [
                    1.0, 2, "3", true, null, [0, 1, 2], { "key": "value" }
                ],
                "map": {
                    "foo": "bar"
                }
            }),
        };

        let start = Instant::now();
        if let Err(err) = program.run(&mut target) {
            panic!("{}", Diagnostic::new(&content).snippets(err));
        }

        println!("{:016?}{}", start.elapsed(), path.to_string_lossy());
    }
}
