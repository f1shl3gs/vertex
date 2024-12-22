use http::{Method, Uri};

use crate::schema::{
    generate_const_string_schema, generate_one_of_schema, InstanceType, SchemaGenerator,
    SchemaObject,
};
use crate::Configurable;

impl Configurable for Uri {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        let mut schema = SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            format: Some("uri"),
            ..Default::default()
        };

        let metadata = schema.metadata();
        metadata.examples = vec![serde_json::Value::String(
            "http://example.com/some/resource".to_string(),
        )];

        schema
    }
}

impl Configurable for Method {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        let mut schema = generate_one_of_schema(vec![
            generate_const_string_schema("OPTIONS".to_string()),
            generate_const_string_schema("GET".to_string()),
            generate_const_string_schema("POST".to_string()),
            generate_const_string_schema("PUT".to_string()),
            generate_const_string_schema("DELETE".to_string()),
            generate_const_string_schema("HEAD".to_string()),
            generate_const_string_schema("TRACE".to_string()),
            generate_const_string_schema("CONNECT".to_string()),
            generate_const_string_schema("PATCH".to_string()),
        ]);
        let metadata = schema.metadata();
        metadata.examples = vec![
            serde_json::Value::String("GET".to_string()),
            serde_json::Value::String("POST".to_string()),
        ];

        schema
    }
}
