use configurable::generate_config_with_schema;
use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use serde::Serialize;

#[allow(clippy::print_stdout)]
#[test]
fn flatten_struct() {
    #[derive(Configurable, Serialize)]
    struct Inner {
        second: String,
        /// third desc
        third: String,
    }

    #[derive(Configurable, Serialize)]
    struct Outer {
        /// first desc
        first: String,

        #[serde(flatten)]
        inner: Inner,
    }

    let root_schema = generate_root_schema::<Outer>();
    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let example = generate_config_with_schema(root_schema);
    println!("{}", example);
}
