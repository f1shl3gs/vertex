use std::process::exit;

use value::value;
use vtl::{Diagnostic, TargetValue, compile};

fn main() {
    let Some(path) = std::env::args().skip(1).next() else {
        println!("Usage: vtl <file>");
        exit(1);
    };

    let script = std::fs::read_to_string(path).unwrap();

    let program = match compile(&script) {
        Ok(program) => program,
        Err(err) => {
            let diagnostic = Diagnostic::new(script);
            let output = diagnostic.snippets(err);
            println!("{output}");
            exit(1);
        }
    };

    // build your own target
    let mut target = TargetValue {
        metadata: value!({}),
        value: value!({}),
    };

    if let Err(err) = program.run(&mut target) {
        let diagnostic = Diagnostic::new(script);
        let output = diagnostic.snippets(err);
        println!("{output}");
        exit(1);
    }

    let output = serde_json::to_string_pretty(&target.value).unwrap();

    println!("{output}");
}
