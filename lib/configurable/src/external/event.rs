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
            "upper" => generate_number_schema::<f64>(),
            "count" => generate_number_schema::<u64>(),
        };
        let requirement = BTreeSet::from(["upper", "count"]);

        Ok(generate_struct_schema(properties, requirement, None))
    }
}

impl Configurable for Quantile {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let properties = indexmap! {
            "quantile" => generate_number_schema::<f64>(),
            "value" => generate_number_schema::<f64>(),
        };
        let requirement = BTreeSet::from(["quantile", "value"]);

        Ok(generate_struct_schema(properties, requirement, None))
    }
}

impl Configurable for MetricValue {
    fn generate_schema(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let histogram_properties = indexmap! {
            "count" => generate_number_schema::<u64>(),
            "sum" => generate_number_schema::<f64>(),
            "buckets" => generate_array_schema::<Bucket>(gen)?,
        };
        let histogram_requirement = BTreeSet::from(["count", "sum", "buckets"]);

        let summary_properties = indexmap! {
            "count" => generate_number_schema::<u64>(),
            "sum" => generate_number_schema::<f64>(),
            "quantiles" => generate_array_schema::<Quantile>(gen)?,
        };
        let summary_requirement = BTreeSet::from(["count", "sum", "quantiles"]);

        Ok(generate_one_of_schema(&[
            generate_number_schema::<f64>(),
            generate_struct_schema(histogram_properties, histogram_requirement, None),
            generate_struct_schema(summary_properties, summary_requirement, None),
        ]))
    }
}
