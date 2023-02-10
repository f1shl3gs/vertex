use configurable::example::Visitor;
use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use serde::Serialize;

#[allow(dead_code)]
#[allow(clippy::print_stdout)]
#[test]
fn flatten_struct() {
    #[derive(Configurable, Serialize)]
    #[serde(rename_all = "lowercase", tag = "mode")]
    enum Inner {
        Tcp { addr: String, tls: String },
        Udp { addr: String },
    }

    #[derive(Configurable, Serialize)]
    struct Config {
        /// first desc
        first: String,

        #[serde(flatten)]
        inner: Inner,
    }

    let root_schema = generate_root_schema::<Config>().expect("generate schema success");
    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let visitor = Visitor::new(root_schema);
    let example = visitor.example();
    println!("{}", example);
}
