use configurable::generate_config_with_schema;
use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use serde::{Deserialize, Serialize};

#[allow(clippy::print_stdout)]
#[test]
fn enum_with_tag() {
    let root_schema = generate_root_schema::<Outer>().expect("generate schema success");

    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let example = generate_config_with_schema(root_schema);
    println!("{}", example);

    #[derive(Configurable, Deserialize, Serialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    enum Inner {
        Basic { user: String, password: String },
        Bearer { token: String },
    }

    #[derive(Configurable, Deserialize, Serialize)]
    struct Outer {
        inner: Inner,
        other: String,
    }
}
