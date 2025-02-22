use std::collections::{BTreeMap, BTreeSet};

use event::tags::{Key, Tags, Value};
use event::{Bucket, MetricValue, Quantile};
use indexmap::indexmap;

use crate::schema::{
    InstanceType, SchemaGenerator, SchemaObject, generate_number_schema, generate_one_of_schema,
    generate_struct_schema,
};
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
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        let properties = indexmap! {
            "upper" => generate_number_schema::<f64>(),
            "count" => generate_number_schema::<u64>(),
        };
        let requirement = BTreeSet::from(["upper", "count"]);

        generate_struct_schema(properties, requirement, None)
    }
}

impl Configurable for Quantile {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        let properties = indexmap! {
            "quantile" => generate_number_schema::<f64>(),
            "value" => generate_number_schema::<f64>(),
        };
        let requirement = BTreeSet::from(["quantile", "value"]);

        generate_struct_schema(properties, requirement, None)
    }
}

impl Configurable for MetricValue {
    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        let histogram_properties = indexmap! {
            "count" => u64::generate_schema(generator),
            "sum" => f64::generate_schema(generator),
            "buckets" => Vec::<Bucket>::generate_schema(generator),
        };
        let histogram_requirement = BTreeSet::from(["count", "sum", "buckets"]);

        let summary_properties = indexmap! {
            "count" => u64::generate_schema(generator),
            "sum" => f64::generate_schema(generator),
            "quantiles" => Vec::<Quantile>::generate_schema(generator),
        };
        let summary_requirement = BTreeSet::from(["count", "sum", "quantiles"]);

        generate_one_of_schema(vec![
            f64::generate_schema(generator),
            generate_struct_schema(histogram_properties, histogram_requirement, None),
            generate_struct_schema(summary_properties, summary_requirement, None),
        ])
    }
}
