// #![deny(warnings)]

use configurable_derive::configurable_component;

#[allow(clippy::print_stdout)]
#[test]
fn test_generate() {
    #[configurable_component(sink, name = "some")]
    #[derive(Default)]
    pub struct SomeConfig {
        #[configurable(required)]
        foo: String,
    }

    // assert!("some" == SomeConfig::NAME);

    let schema = configurable::schema::generate_root_schema::<SomeConfig>().unwrap();
    let json = serde_json::to_string_pretty(&schema)
        .expect("rendering root schema to JSON should not fail");

    println!("{}", json);
}
