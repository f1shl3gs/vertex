use std::num::NonZeroI64;

use configurable::Configurable;
use serde::Serialize;

#[allow(clippy::print_stdout)]
fn gen<T: Configurable + Serialize + Sized>() {
    let schema = configurable::schema::generate_root_schema::<T>();

    let json = serde_json::to_string_pretty(&schema)
        .expect("rendering root schema to JSON should not fail");

    println!("{}", json)
}

#[allow(dead_code)]
#[test]
fn derive_gen() {
    #[derive(Serialize, Configurable)]
    #[configurable(description = "inner desc")]
    struct Inner {
        float: f64,
    }

    #[derive(Serialize, Configurable)]
    struct External {
        first: i32,
        second: bool,
    }

    /// foo is root
    ///
    /// blah blah
    #[derive(Serialize, Configurable)]
    struct Foo {
        #[configurable(required, description = "number field", default = 12)]
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

    #[derive(Serialize, Configurable)]
    enum Variant {
        None,
        Internal {
            #[configurable(description = "first desc", required)]
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

    gen::<Foo>();
}
