use std::collections::BTreeMap;

use configurable::example::Visitor;
use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use serde::{Deserialize, Serialize};

#[allow(clippy::print_stdout)]
#[test]
fn empty_map() {
    #[derive(Configurable, Default, Deserialize, Serialize)]
    struct Outer {
        map: BTreeMap<String, String>,
    }

    let root_schema = generate_root_schema::<Outer>().unwrap();

    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let visitor = Visitor::new(root_schema);
    let example = visitor.example();

    println!("{}", example)
}
