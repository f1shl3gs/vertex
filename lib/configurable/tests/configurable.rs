#![allow(dead_code)]

use std::num::NonZeroI64;

use configurable::generate_config_with_schema;
use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use serde::Deserialize;

#[derive(Deserialize, Configurable)]
#[configurable(description = "inner desc")]
struct Inner {
    float: f64,
}

#[derive(Deserialize, Configurable)]
struct External {
    first: i32,
    second: bool,
}

/// foo is root
///
/// blah blah
#[derive(Deserialize, Configurable)]
struct Foo {
    #[configurable(description = "number field", default = 12)]
    field_num: u32,
    #[configurable(default = "aaaaa")]
    filed_str: String,
    non_num: NonZeroI64,

    /// filed bool is
    #[configurable(default = true)]
    field_bool: bool,

    /// inner is inner type
    inner: Inner,

    variant: Variant,

    optional: Option<String>,
}

#[derive(Deserialize, Configurable)]
enum Variant {
    None,
    Internal {
        #[configurable(description = "first desc")]
        first: String,
        #[configurable(default = 12)]
        second: u32,

        /// uri for ....
        #[configurable(format = "uri")]
        uri: String,
    },
    External(External),
    // Tuple((i32, i32))
}

#[allow(dead_code)]
#[test]
fn generate() {
    let root_schema = generate_root_schema::<Foo>();

    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{text}");

    let example = generate_config_with_schema(root_schema);
    println!("{example}");

    let _ = serde_yaml::from_str::<Foo>(&example).unwrap();
}
