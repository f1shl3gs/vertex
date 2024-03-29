use value::Value;
use vtl::{compile, Diagnostic, TargetValue};

fn run(content: String) {
    let mut program = match compile(&content) {
        Ok(program) => program,
        Err(err) => {
            let diagnostic = Diagnostic::new(content);
            let output = diagnostic.snippets(err);
            panic!("{}", output);
        }
    };

    let mut target = TargetValue {
        metadata: Value::Object(Default::default()),
        value: Value::Object(Default::default()),
    };

    if let Err(err) = program.run(&mut target) {
        let diagnostic = Diagnostic::new(content);
        let output = diagnostic.snippets(err);
        panic!("{}", output);
    }
}

#[allow(clippy::print_stdout)]
#[test]
fn run_scripts() {
    let paths = glob::glob("tests/**/*.vtl").unwrap();
    for path in paths.flatten() {
        println!("{:?}", path);
        let content = std::fs::read_to_string(path).unwrap();

        run(content);
    }
}
