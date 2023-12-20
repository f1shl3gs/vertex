use configurable::schema::generate_root_schema;
use configurable::Examplar;
use configurable_derive::Configurable;
use serde::Serialize;

#[allow(dead_code)]
#[allow(clippy::print_stdout)]
#[test]
fn gen() {
    #[derive(Configurable, Serialize)]
    struct Config {
        first: String,
        #[serde(skip)]
        second: i32,
        third: i32,
    }

    let root_schema = generate_root_schema::<Config>().unwrap();

    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let example = Examplar::new(root_schema).generate();

    println!("{}", example)
}
