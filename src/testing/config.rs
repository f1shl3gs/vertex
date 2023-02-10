use configurable::{generate_config, Configurable};

pub fn test_generate_config<T>()
where
    for<'de> T: Configurable + serde::Deserialize<'de>,
{
    let cfg = generate_config::<T>();
    if let Err(err) = serde_yaml::from_str::<T>(&cfg) {
        panic!(
            "{}\n\n----------------- Generated config -------------\n{}\n",
            err, cfg
        )
    }
}
