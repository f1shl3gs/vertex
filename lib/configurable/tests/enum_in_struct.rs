use configurable::schema::generate_root_schema;
use configurable::{configurable_component, generate_config_with_schema, Configurable};
use serde::{Deserialize, Serialize};

#[derive(Configurable, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
enum Stream {
    /// Stdout
    #[default]
    Stdout,
    /// stderr stream.
    Stderr,
}

#[derive(Configurable, Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    /// JSON
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

#[allow(clippy::print_stdout)]
#[test]
fn enum_in_struct() {
    let root_schema = generate_root_schema::<ConsoleSinkConfig>().expect("generate schema success");

    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let example = generate_config_with_schema(root_schema);
    println!("{}", example);

    let _ = serde_yaml::from_str::<ConsoleSinkConfig>(&example).unwrap();
}
