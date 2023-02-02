use configurable::Configurable;
use serde::Serialize;
use std::num::NonZeroI64;

#[allow(clippy::print_stdout)]
fn gen<T: Configurable + Serialize + Sized>() {
    let schema =
        configurable::schema::generate_root_schema::<T>().expect("generate root schema failed");

    // let m = schema.schema.metadata();
    // m.deprecated

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

    #[derive(Serialize, Configurable)]
    struct Foo {
        #[configurable(required, description = "number field")]
        field_num: u32,
        #[configurable()]
        filed_str: String,
        non_num: NonZeroI64,

        inner: Inner,

        variant: Variant,
    }

    #[derive(Serialize, Configurable)]
    enum Variant {
        None,
        Internal {
            #[configurable(description = "first desc", required)]
            first: String,
            second: u32,
        },
        External(External),
        // Tuple((i32, i32))
    }

    gen::<Foo>();
}
