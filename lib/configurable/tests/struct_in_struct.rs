use configurable_derive::Configurable;
use serde::Serialize;

#[allow(clippy::print_stdout)]
#[test]
fn struct_in_struct() {
    #[derive(Configurable, Serialize)]
    struct Inner {
        foo: String,
        bar: String,
    }

    #[derive(Configurable, Serialize)]
    struct Outer {
        /// inner desc
        inner: Inner,

        boolean: bool,
    }

    let example = configurable::generate_config::<Outer>();
    println!("{}", example)
}
