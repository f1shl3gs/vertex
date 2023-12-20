use std::collections::BTreeMap;

use configurable::schema::generate_root_schema;
use configurable::Examplar;
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

    let example = Examplar::new(root_schema).generate();

    println!("{}", example);

    let _ = serde_yaml::from_str::<Outer>(&example).unwrap();
}
