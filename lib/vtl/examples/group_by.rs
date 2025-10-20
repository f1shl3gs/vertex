use value::value;
use vtl::{Diagnostic, TargetValue, compile};

fn main() {
    let mut target = TargetValue {
        metadata: value!({}),
        value: value!([
            {
                "service": "foo",
                "server": "127.0.0.1",
            },
            {
                "service": "bar",
                "server": "127.0.0.2",
            },
            {
                "service": "foo",
                "server": "127.0.0.3",
            },
        ]),
    };

    let script = r#"
grouped = {}

for index, item in . {
    servers = get(grouped, [ item.service ])
    if is_null(servers) {
        servers = []
    }

    grouped = set(grouped, [ item.service ], push( servers, item.server ))
}

# returned value
grouped

"#;

    let diagnostic = Diagnostic::new(script);

    let program = match compile(script) {
        Ok(program) => program,
        Err(err) => {
            println!("{}", diagnostic.snippets(err));
            return;
        }
    };

    match program.run(&mut target) {
        Ok(value) => {
            let output = serde_json::to_string_pretty(&value).unwrap();
            println!("{output}");
        }
        Err(err) => {
            println!("{}", diagnostic.snippets(err));
        }
    }
}
