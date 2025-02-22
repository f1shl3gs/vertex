use configurable::generate_config_with_schema;
use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use serde::{Deserialize, Serialize};

#[allow(clippy::print_stdout)]
#[allow(dead_code)]
#[test]
fn generate() {
    #[derive(Configurable, Deserialize, Serialize)]
    struct OneStruct {
        one: String,
        oah: i32,
    }

    #[derive(Configurable, Deserialize, Serialize)]
    struct TwoStruct {
        two: i32,
        three: i32,
    }

    #[derive(Configurable, Serialize)]
    #[serde(tag = "type", rename_all = "lowercase")]
    enum Config {
        One(OneStruct),
        Two(TwoStruct),
        Three(String), // Two(TwoStruct),
                       // Three(Three)
    }

    let root_schema = generate_root_schema::<Config>();

    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let example = generate_config_with_schema(root_schema);
    println!("{}", example)
}
