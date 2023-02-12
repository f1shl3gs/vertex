use configurable::example::Visitor;
use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[allow(clippy::print_stdout)]
#[test]
fn flatten_struct() {
    #[derive(Configurable, Serialize, Deserialize)]
    struct UnixDetail {
        path: String,
    }

    #[derive(Configurable, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase", tag = "mode")]
    enum Protocol {
        /// unix variant
        Unix(UnixDetail),

        /// tcp variant
        Tcp { addr: String, tls: String },

        /// udp variant
        Udp { addr: String },
    }

    #[derive(Configurable, Serialize, Deserialize)]
    struct Config {
        /// common desc
        common: String,

        #[serde(flatten)]
        inner: Protocol,
    }

    let root_schema = generate_root_schema::<Config>().expect("generate schema success");
    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let visitor = Visitor::new(root_schema);
    let example = visitor.example();
    println!("{}", example);

    let _n = serde_yaml::from_str::<Config>(&example).unwrap();
}
