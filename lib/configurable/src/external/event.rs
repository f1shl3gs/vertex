use std::collections::{BTreeMap, BTreeSet};

use event::tags::{Key, Tags, Value};
use event::{Bucket, MetricValue, Quantile};
use indexmap::indexmap;

use crate::schema::{
    generate_array_schema, generate_map_schema, generate_number_schema, generate_one_of_schema,
    generate_string_schema, generate_struct_schema, InstanceType, SchemaGenerator, SchemaObject,
};
use crate::{Configurable, ConfigurableString, GenerateError};

impl Configurable for Key {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(generate_string_schema())
    }
}

impl ConfigurableString for Key {}

impl Configurable for Value {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(SchemaObject {
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
        })
    }
}

impl Configurable for Tags {
    fn generate_schema(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        generate_map_schema::<BTreeMap<Key, Value>>(gen)
    }
}

impl Configurable for Bucket {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let properties = indexmap! {
            "upper".to_string() => generate_number_schema::<f64>(),
            "count".to_string() => generate_number_schema::<u64>(),
        };
        let requirement = BTreeSet::from(["upper".to_string(), "count".to_string()]);

        Ok(generate_struct_schema(properties, requirement, None))
    }
}

impl Configurable for Quantile {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let properties = indexmap! {
            "quantile".to_string() => generate_number_schema::<f64>(),
            "value".to_string() => generate_number_schema::<f64>(),
        };
        let requirement = BTreeSet::from(["quantile".to_string(), "value".to_string()]);

        Ok(generate_struct_schema(properties, requirement, None))
    }
}

impl Configurable for MetricValue {
    fn generate_schema(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let histogram_properties = indexmap! {
            "count".to_string() => generate_number_schema::<u64>(),
            "sum".to_string() => generate_number_schema::<f64>(),
            "buckets".to_string() => generate_array_schema::<Bucket>(gen)?,
        };
        let histogram_requirement = BTreeSet::from([
            "count".to_string(),
            "sum".to_string(),
            "buckets".to_string(),
        ]);

        let summary_properties = indexmap! {
            "count".to_string() => generate_number_schema::<u64>(),
            "sum".to_string() => generate_number_schema::<f64>(),
            "quantiles".to_string() => generate_array_schema::<Quantile>(gen)?,
        };
        let summary_requirement = BTreeSet::from([
            "count".to_string(),
            "sum".to_string(),
            "quantiles".to_string(),
        ]);

        Ok(generate_one_of_schema(&[
            generate_number_schema::<f64>(),
            generate_struct_schema(histogram_properties, histogram_requirement, None),
            generate_struct_schema(summary_properties, summary_requirement, None),
        ]))
    }
}
