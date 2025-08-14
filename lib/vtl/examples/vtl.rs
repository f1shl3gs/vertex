use std::process::exit;

use value::value;
use vtl::{Diagnostic, TargetValue, compile};

fn main() {
    let mut args = std::env::args();
    let Some(path) = args.nth(1) else {
        println!("Usage: vtl <file> [input]");
        exit(1);
    };

    let value = match args.next() {
        Some(path) => {
            let content = std::fs::read(path).unwrap();
            serde_json::from_slice(&content).unwrap()
        }
        None => value!({}),
    };
    let mut target = TargetValue {
        metadata: value!({}),
        value,
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

    if let Err(err) = program.run(&mut target) {
        let diagnostic = Diagnostic::new(script);
        let output = diagnostic.snippets(err);
        println!("{output}");
        exit(1);
    }

    let output = serde_json::to_string_pretty(&target.value).unwrap();

    println!("{output}");
}
