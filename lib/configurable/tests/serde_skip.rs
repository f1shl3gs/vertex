use configurable::generate_config_with_schema;
use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use serde::Serialize;

#[allow(dead_code)]
#[allow(clippy::print_stdout)]
#[test]
fn generate() {
    #[derive(Configurable, Serialize)]
    struct Config {
        first: String,
        #[serde(skip)]
        second: i32,
        third: i32,
    }

    let root_schema = generate_root_schema::<Config>();

    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{text}");

    let example = generate_config_with_schema(root_schema);
    println!("{example}")
}
