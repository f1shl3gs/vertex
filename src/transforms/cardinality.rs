use async_trait::async_trait;
use bloom::{BloomFilter, ASMS};
use event::Event;
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, Transform};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

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
    //
    pub limit: usize,
    pub action: LimitExceededAction,
}

#[async_trait]
#[typetag::serde(name = "cardinality")]
impl TransformConfig for CardinalityConfig {
    async fn build(&self, _ctx: &TransformContext) -> crate::Result<Transform> {
        Ok(Transform::function(Cardinality::new(self.limit)))
    }

    fn input_type(&self) -> DataType {
        DataType::Metric
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn transform_type(&self) -> &'static str {
        "cardinality"
    }
}

struct TagValueSet {
    elements: usize,
    filter: BloomFilter,
}

impl Debug for TagValueSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bloom")
    }
}

impl TagValueSet {
    pub fn new(limit: usize) -> Self {
        let bits = (5000 * 1024) / 8;
        let hashes = bloom::optimal_num_hashes(bits, limit as u32);

        Self {
            elements: 0,
            filter: BloomFilter::with_size(bits, hashes),
        }
    }

    #[inline]
    fn contains(&self, value: &str) -> bool {
        self.filter.contains(&value)
    }

    #[inline]
    const fn len(&self) -> usize {
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

struct Cardinality {
    limit: usize,
    action: LimitExceededAction,
    accepted_tags: HashMap<String, TagValueSet>,
}

impl Clone for Cardinality {
    fn clone(&self) -> Self {
        Self {
            limit: self.limit,
            action: self.action,
            accepted_tags: HashMap::new(),
        }
    }
}

impl Cardinality {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            action: LimitExceededAction::DropEvent,
            accepted_tags: HashMap::new(),
        }
    }

    /// Takes in key and a value corresponding to a tag on an incoming Metric
    /// event. If that value is already part of set of accepted values for that
    /// key, then simply retruns true. If that value is not yet part of the
    /// accepted values for that key, checks whether we have hit the limit
    /// for that key yet and if not adds the value to the set of accepted values
    /// for the key and returns true, otherwise returns false. A false return
    /// value indicates to the caller that the value is not accepted for this
    /// key, and the configured limit_execeed_action should be taken.
    fn try_accept_tag(&mut self, key: &str, value: &str) -> bool {
        if !self.accepted_tags.contains_key(key) {
            self.accepted_tags
                .insert(key.to_string(), TagValueSet::new(self.limit));
        }

        let set = self.accepted_tags.get_mut(key).unwrap();
        if set.contains(value) {
            // Tag value has already been accepted, nothing more to do
            return true;
        }

        // Tag value not yet part of the accepted set
        if set.len() < self.limit {
            // accept the new value
            set.insert(value);
            if set.len() == self.limit {
                // emit limit reached event
            }

            true
        } else {
            // New tag value is rejected
            false
        }
    }
}

impl FunctionTransform for Cardinality {
    fn transform(&mut self, output: &mut Vec<Event>, event: Event) {
        let metric = event.as_metric();

        for (k, v) in &metric.tags {
            if !self.try_accept_tag(k, v) {
                // rejected
                return;
            }
        }

        output.push(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_value_set() {
        let mut set = TagValueSet::new(10);
        assert_eq!(set.len(), 0);
        assert!(!set.contains("foo"));

        assert!(set.insert("foo"));
        assert!(set.contains("foo"));
        assert_eq!(set.len(), 1);

        assert!(set.insert("bar"));
        assert!(set.contains("bar"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_tag_value_set_limit() {
        let total = 50;
        let mut set = TagValueSet::new(total);
        for i in 0..total {
            let val = format!("{}", i);
            assert!(set.insert(&val));
        }

        assert_eq!(set.len(), total);

        for i in 0..total {
            let val = format!("{}", i);
            assert!(!set.insert(&val))
        }
    }
}
