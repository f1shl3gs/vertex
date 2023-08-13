use configurable::example::Visitor;
use configurable::schema::generate_root_schema;
use configurable_derive::{configurable_component, Configurable};
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

#[allow(clippy::print_stdout)]
#[test]
fn flatten_enum() {
    /// Controls the approach token for tracking tag cardinality.
    #[derive(Configurable, Copy, Clone, Debug, Deserialize, Serialize)]
    #[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
    pub enum Mode {
        /// Tracks cardinality exactly.
        ///
        /// This mode has higher memory requirements than `probabilistic`, but
        /// never falsely outputs metrics with new tags after the limit has
        /// been hit.
        Exact,

        /// Tracks cardinality probabilistically.
        ///
        /// This mode has lower memory requirements than `exact`, but may occasionally
        /// allow metric events to pass through the transform even when they contain
        /// new tags that exceed the configured limit. The rate at which this happens
        /// can be controlled by changing the value of `cache_size_per_tag`
        Probabilistic {
            #[configurable(required)]
            cache_size_per_tag: usize,
        },
    }

    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Configurable, Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum LimitExceededAction {
        #[default]
        Drop,

        DropTag,
    }

    #[configurable_component(transform, name = "cardinality")]
    #[derive(Copy, Clone, Debug)]
    #[serde(deny_unknown_fields)]
    struct Config {
        /// How many distict values for any given key.
        #[configurable(required)]
        limit: usize,

        /// The behavior of limit exceeded action.
        #[serde(default)]
        action: LimitExceededAction,

        mode: Mode,
    }

    let root_schema = generate_root_schema::<Config>().expect("generate schema success");
    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let visitor = Visitor::new(root_schema);
    let example = visitor.example();
    println!("{}", example);
}
