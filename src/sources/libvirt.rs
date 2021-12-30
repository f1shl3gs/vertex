use crate::config::{DataType, GenerateConfig, SourceConfig, SourceContext};
use crate::sources::Source;

#[derive(Debug, Deserialize, Serialize)]
struct LibvirtSourceConfig {}

impl GenerateConfig for LibvirtSourceConfig {
    fn generate_config() -> serde_yaml::Value {
        todo!()
    }
}

inventory::submit! {
    SourceDescription::new::<LibvirtSourceConfig>("libvirt")
}

#[async_trait::async_trait]
#[typetag::serde(name = "libvirt")]
impl SourceConfig for LibvirtSourceConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        todo!()
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "libvirt"
    }
}
