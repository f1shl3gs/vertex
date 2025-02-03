use std::collections::BTreeSet;

use configurable::schema::{
    ArrayValidation, InstanceType, Metadata, SchemaGenerator, SchemaObject, SingleOrVec,
};
use configurable::Configurable;
use indexmap::IndexMap;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};

/// List of allowed/denied origin IP networks. IP addresses must be in CIDR notation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IpAccessConfig {
    allow: Vec<IpNet>,
    deny: Vec<IpNet>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
struct IpAllow(Vec<IpNet>);

impl Configurable for IpAccessConfig {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        let mut properties = IndexMap::new();
        properties.insert("allow", generate_ipnet_list_schema());
        properties.insert("deny", generate_ipnet_list_schema());

        configurable::schema::generate_struct_schema(properties, BTreeSet::new(), None)
    }
}

fn generate_ipnet_list_schema() -> SchemaObject {
    let examples = serde_json::from_str(
        r#"    [
        "192.168.0.0/16",
        "127.0.0.1/32",
        "::1/128",
        "9876:9ca3:99ab::23/128"
    ]"#,
    )
    .unwrap();

    SchemaObject {
        instance_type: Some(InstanceType::Array.into()),
        array: Some(Box::new(ArrayValidation {
            items: Some(SingleOrVec::Single(Box::new(
                SchemaObject {
                    instance_type: Some(InstanceType::String.into()),
                    metadata: Some(Box::new(Metadata {
                        examples,
                        ..Default::default()
                    })),
                    ..Default::default()
                }
                .into(),
            ))),
            ..Default::default()
        })),
        ..Default::default()
    }
}
