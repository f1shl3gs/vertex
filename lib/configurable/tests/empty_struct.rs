use configurable::generate_config_with_schema;
use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use serde::{Deserialize, Serialize};

#[allow(clippy::print_stdout)]
#[test]
fn empty_struct() {
    #[derive(Configurable, Serialize, Deserialize)]
    struct Empty {}

    let root_schema = generate_root_schema::<Empty>().expect("generate schema success");
    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let example = generate_config_with_schema(root_schema);
    println!("{}", example);

    let _ = serde_yaml::from_str::<Empty>(&example).unwrap();
}
