use framework::config::GenerateConfig;

pub fn test_generate_config<T>()
where
    for<'de> T: GenerateConfig + serde::Deserialize<'de>,
{
    let cfg = T::generate_config();
    serde_yaml::from_str::<T>(&cfg).expect("Invalid config generated");
}
