use configurable::example::Visitor;
use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use serde::Serialize;

#[allow(clippy::print_stdout)]
#[allow(dead_code)]
#[test]
fn gen() {
    #[derive(Configurable, Serialize)]
    struct Inner {
        a: i32,
        b: i32,
    }

    #[derive(Configurable, Serialize)]
    #[serde(rename_all = "lowercase", untagged)]
    enum Mode {
        /// first desc
        First(i32),
        Second(String),
        Third(Inner),
    }

    #[derive(Configurable, Serialize)]
    #[serde(tag = "type", rename_all = "lowercase")]
    struct Config {
        addr: String,
        mode: Mode,
    }

    let root_schema = generate_root_schema::<Config>().unwrap();

    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let visitor = Visitor::new(root_schema);
    let example = visitor.example();

    println!("{}", example)
}
