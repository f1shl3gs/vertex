#![allow(clippy::print_stdout)]

use value::value;
use vtl::{Diagnostic, TargetValue, compile};

fn main() {
    let script = r#"
.msg, err = parse_json(.msg)
if err != null {
    .err = err
    return
}

.url, err = parse_url("https://example.io?foo=bar")
if err != null {
    .url_err = err
    return
}
.url.fragment = "blah"

.partition = %partition
.offset = %offset

.foo_bar = 1

del(.array)

    "#;

    let diagnostic = Diagnostic::new(script.to_string());

    let program = match compile(script) {
        Ok(program) => program,
        Err(err) => {
            let output = diagnostic.snippets(err);
            println!("{output}");
            return;
        }
    };

    // build your own target
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

    if let Err(err) = program.run(&mut target) {
        let output = diagnostic.snippets(err);
        println!("{output}");
    }

    let output = serde_json::to_string_pretty(&target.value).unwrap();
    println!("{output}");
}
