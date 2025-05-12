use http::{Method, Uri};

use crate::Configurable;
use crate::schema::{InstanceType, SchemaGenerator, SchemaObject};

impl Configurable for Uri {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        let mut schema = SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            format: Some("uri"),
            ..Default::default()
        };

        schema.metadata.examples = vec![serde_json::Value::String(
            "http://example.com/some/resource".to_string(),
        )];

        schema
    }
}

impl Configurable for Method {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        let mut schema = SchemaObject::one_of(
            vec![
                SchemaObject::const_value("OPTIONS"),
                SchemaObject::const_value("GET"),
                SchemaObject::const_value("POST"),
                SchemaObject::const_value("PUT"),
                SchemaObject::const_value("DELETE"),
                SchemaObject::const_value("HEAD"),
                SchemaObject::const_value("TRACE"),
                SchemaObject::const_value("CONNECT"),
                SchemaObject::const_value("PATCH"),
            ],
            None,
        );

        schema.metadata.examples = vec![
            serde_json::Value::String("GET".to_string()),
            serde_json::Value::String("POST".to_string()),
        ];

        schema
    }
}
