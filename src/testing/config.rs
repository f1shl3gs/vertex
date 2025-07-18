use configurable::schema::generate_root_schema;
use configurable::{Configurable, generate_config_with_schema};

pub fn generate_config<T>()
where
    for<'de> T: Configurable + serde::Deserialize<'de>,
{
    let root_schema = generate_root_schema::<T>();
    let schema = serde_json::to_string_pretty(&root_schema).expect("serialize root schema success");

    let cfg = generate_config_with_schema(root_schema);

    if let Err(err) = serde_yaml::from_str::<T>(&cfg) {
        panic!(
            "Deserialize error: {err}\n\n----------------- Generated config -------------\n{cfg}\n{schema}\n"
        )
    }
}
