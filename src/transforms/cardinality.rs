use serde::{Deserialize, Serialize};
use bloom::{ASMS, BloomFilter};
use async_trait::async_trait;
use std::borrow::{Cow};
use crate::config::{TransformConfig, GlobalOptions, DataType};
use crate::transforms::{Transform, FunctionTransform};
use event::Event;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "limit_exceeded_action", rename_all = "snake_case")]
pub enum LimitExceededAction {
    DropEvent,
    DropTag,
}

impl Default for LimitExceededAction {
    fn default() -> Self {
        Self::DropEvent
    }
}


#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct CardinalityConfig {
    pub limit: usize,
    pub action: LimitExceededAction,
}

#[async_trait]
#[typetag::serde(name = "cardinality")]
impl TransformConfig for CardinalityConfig {
    async fn build(&self, globals: &GlobalOptions) -> crate::Result<Transform> {
        Ok(Transform::function(Cardinality::new(self.limit)))
    }

    fn input_type(&self) -> DataType {
        todo!()
    }

    fn output_type(&self) -> DataType {
        todo!()
    }

    fn transform_type(&self) -> &'static str {
        todo!()
    }
}

struct LabelValueSet {
    elements: usize,
    filter: BloomFilter,
}

impl LabelValueSet {
    pub fn new(limit: usize) -> Self {
        let bits = (5000 * 1024) / 8;
        let hashes = bloom::optimal_num_hashes(bits, limit as u32);

        Self {
            elements: 0,
            filter: BloomFilter::with_size(bits, hashes),
        }
    }

    #[inline]
    fn contains(self, value: &str) -> bool {
        self.filter.contains(&value)
    }

    #[inline]
    fn len(&self) -> usize {
        self.elements
    }

    fn insert(&mut self, val: &str) -> bool {
        let inserted = self.filter.insert(&val);

        if inserted {
            self.elements += 1;
        }

        inserted
    }

    fn exceed(self) -> bool {
        todo!()
    }
}

// #[derive(Debug)]
struct Cardinality {
    limit: usize,
    action: LimitExceededAction,
    accepted_labels: HashMap<String, LabelValueSet>,
}

impl Clone for Cardinality {
    fn clone(&self) -> Self {
        Self {
            limit: self.limit,
            action: self.action,
            accepted_labels: HashMap::new(),
        }
    }
}

impl Cardinality {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            action: LimitExceededAction::DropEvent,
            accepted_labels: HashMap::new(),
        }
    }

    fn insert(&mut self, value: Cow<'_, String>) -> bool {
        todo!()
    }

    fn exceeded(&mut self, key: String, value: String) -> bool {
        if !self.accepted_labels.contains_key(&key) {
            return true;
        }
/*
        let value_set = match self.accepted_labels.get_mut(&key) {
            Some(value_set) => value_set,
            None => {
                let mut vs = LabelValueSet::new(self.limit);
                self.accepted_labels.insert(key, vs);
                vs.borrow_mut()
            }
        };

        if value_set.contains(&value) {
            false
        }

        if value_set.len() >= self.limit {
            true
        }

        value_set.insert(&value);*/
        false
    }
}

impl FunctionTransform for Cardinality {
    fn transform(&mut self, output: &mut Vec<Event>, event: Event) {
        let metric = event.as_metric();

    }
}

/*
impl TaskTransform for Cardinality {
    fn transform(
        self: Box<Self>,
        task: Pin<Box<dyn Stream<Item=Event> + Send>>,
    ) -> Pin<Box<dyn Stream<Item=Event> + Send>> {
        todo!()
    }
}*/