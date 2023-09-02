use configurable::example::Visitor;
use configurable::schema::generate_root_schema;
use configurable::{configurable_component, Configurable};
use serde::{Deserialize, Serialize};

#[allow(clippy::print_stdout)]
#[test]
fn enum_in_struct() {
    let root_schema = generate_root_schema::<ConsoleSinkConfig>().expect("generate schema success");

    // let text = serde_json::to_string_pretty(&root_schema).unwrap();
    // println!("{}", text);

    let visitor = Visitor::new(root_schema);

    let example = visitor.example();
    println!("{}", example);

    #[derive(Configurable, Debug, Deserialize, Serialize, Default)]
    #[serde(rename_all = "lowercase")]
    enum Stream {
        #[default]
        Stdout,
        /// stderr stream.
        Stderr,
    }

    #[derive(Configurable, Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
    #[serde(rename_all = "snake_case")]
    pub enum Encoding {
        Json,
        #[configurable(description = "text desc")]
        Text,
    }

    #[derive(Configurable, Serialize, Debug, Deserialize)]
    struct EncodingConfig {
        #[configurable(required)]
        encoding: Encoding,

        #[serde(flatten)]
        flatten: Flatten,
    }

    #[derive(Configurable, Deserialize, Serialize, Debug)]
    struct Flatten {
        first: String,
        second: String,
    }

    #[configurable_component(sink, name = "console")]
    #[serde(deny_unknown_fields)]
    pub struct ConsoleSinkConfig {
        /// The standard stream to write to.
        #[serde(default)]
        stream: Stream,

        encoding: EncodingConfig,
    }
}
