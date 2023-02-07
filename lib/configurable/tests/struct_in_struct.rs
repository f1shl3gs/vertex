use serde::Serialize;
use configurable_derive::Configurable;

#[test]
fn struct_in_struct() {
    #[derive(Configurable, Serialize)]
    struct Inner {
        foo: String,
        bar: String
    }

    #[derive(Configurable, Serialize)]
    struct Outer {
        /// inner desc
        inner: Inner,

        boolean: bool
    }

    let example = configurable::generate_example::<Outer>();
    println!("{}", example)
}