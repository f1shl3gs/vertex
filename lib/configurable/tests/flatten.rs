use configurable::schema::generate_root_schema;
use configurable::Examplar;
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

    let root_schema = generate_root_schema::<Outer>().expect("generate schema success");
    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let example = Examplar::new(root_schema).generate();
    println!("{}", example);
}
