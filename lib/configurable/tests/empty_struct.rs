use configurable::schema::generate_root_schema;
use configurable::Examplar;
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

    let example = Examplar::new(root_schema).generate();
    println!("{}", example);

    let _ = serde_yaml::from_str::<Empty>(&example).unwrap();
}
