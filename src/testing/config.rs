use framework::config::GenerateConfig;

pub fn test_generate_config<T>()
where
    for<'de> T: GenerateConfig + serde::Deserialize<'de>,
{
    let cfg = T::generate_config();
    if let Err(err) = serde_yaml::from_str::<T>(&cfg) {
        panic!(
            "{}\n\n----------------- Generated config -------------{}\n",
            err, cfg
        )
    }
}
