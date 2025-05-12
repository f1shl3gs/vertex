use std::collections::BTreeMap;

use event::tags::{Key, Tags, Value};
use event::{Bucket, MetricValue, Quantile};

use crate::schema::{InstanceType, SchemaGenerator, SchemaObject};
use crate::{Configurable, ConfigurableString};

impl Configurable for Key {
    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        String::generate_schema(generator)
    }
}

impl ConfigurableString for Key {}

impl Configurable for Value {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        SchemaObject {
            instance_type: Some(
                vec![
                    InstanceType::Boolean,
                    InstanceType::Integer,
                    InstanceType::Number,
                    InstanceType::String,
                    InstanceType::Array,
                ]
                .into(),
            ),
            ..Default::default()
        }
    }
}

impl Configurable for Tags {
    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        BTreeMap::<Key, Value>::generate_schema(generator)
    }
}

impl Configurable for Bucket {
    fn generate_schema(g: &mut SchemaGenerator) -> SchemaObject {
        let mut schema = SchemaObject::new_object(None);

        schema.insert_property("upper", true, None, f64::generate_schema(g));
        schema.insert_property("count", true, None, u64::generate_schema(g));

        schema
    }
}

impl Configurable for Quantile {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        let mut schema = SchemaObject::new_object(None);

        schema.insert_property("quantile", true, None, f64::generate_schema(_gen));
        schema.insert_property("value", true, None, f64::generate_schema(_gen));

        schema
    }
}

impl Configurable for MetricValue {
    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        let mut subschemas = Vec::with_capacity(3);

        subschemas.push(f64::generate_schema(generator));
        subschemas.push({
            let mut subschema = SchemaObject::new_object(None);
            subschema.insert_property("count", true, None, u64::generate_schema(generator));
            subschema.insert_property("sum", true, None, f64::generate_schema(generator));
            subschema.insert_property(
                "buckets",
                true,
                None,
                Vec::<Bucket>::generate_schema(generator),
            );

            subschema
        });

        subschemas.push({
            let mut subschema = SchemaObject::new_object(None);
            subschema.insert_property("count", true, None, u64::generate_schema(generator));
            subschema.insert_property("sum", true, None, f64::generate_schema(generator));
            subschema.insert_property(
                "quantiles",
                true,
                None,
                Vec::<Quantile>::generate_schema(generator),
            );

            subschema
        });

        SchemaObject::one_of(subschemas, None)
    }
}
