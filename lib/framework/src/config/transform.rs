use std::collections::HashSet;
use std::fmt::Debug;

use async_trait::async_trait;
use configurable::NamedComponent;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::config::{ComponentKey, DataType, ExpandType, GlobalOptions, Output};
use crate::transform::Noop;

#[derive(Debug, Default)]
pub struct TransformContext {
    // This is optional because currently there are a lot of places we use `TransformContext`
    // that may not have the relevant data available (e.g. tests). In the furture it'd be
    // nice to make it required somehow.
    pub key: Option<ComponentKey>,
    pub globals: GlobalOptions,
}

impl TransformContext {
    pub fn new_with_globals(globals: GlobalOptions) -> Self {
        Self {
            globals,
            ..Default::default()
        }
    }
}

/// Generalized interface for describing and building transform components.
#[async_trait]
#[typetag::serde(tag = "type")]
pub trait TransformConfig: NamedComponent + Debug + Send + Sync {
    /// Builds the transform with the given context.
    async fn build(&self, cx: &TransformContext) -> crate::Result<crate::Transform>;

    /// Gets the input configuration for this transform.
    fn input_type(&self) -> DataType;

    /// Gets the list of outputs exposed by this transform.
    fn outputs(&self) -> Vec<Output>;

    /// Returns true if the transform is able to be run across multiple tasks simultaneously
    /// with no concerns around statfulness, ordering, etc
    fn enable_concurrency(&self) -> bool {
        false
    }

    /// Allows to detect if a transform can be embedded in another transform.
    /// It's used by the pipelines transform for now
    fn nestable(&self, _parents: &HashSet<&'static str>) -> bool {
        true
    }

    /// Allows a transform configuration to expand itself into multiple "child"
    /// transformations to replace it. this allows a transform to act as a
    /// macro for various patterns
    fn expand(
        &mut self,
    ) -> crate::Result<Option<(IndexMap<String, Box<dyn TransformConfig>>, ExpandType)>> {
        Ok(None)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransformOuter<T> {
    pub inputs: Vec<T>,

    #[serde(flatten)]
    pub inner: Box<dyn TransformConfig>,
}

impl<T> TransformOuter<T> {
    pub fn with_inputs<U>(self, inputs: Vec<U>) -> TransformOuter<U> {
        TransformOuter {
            inputs,
            inner: self.inner,
        }
    }
}

impl TransformOuter<String> {
    pub(crate) fn expand(
        mut self,
        key: ComponentKey,
        parent_types: &HashSet<&'static str>,
        transforms: &mut IndexMap<ComponentKey, TransformOuter<String>>,
        expansions: &mut IndexMap<ComponentKey, Vec<ComponentKey>>,
    ) -> Result<(), String> {
        if !self.inner.nestable(parent_types) {
            return Err(format!(
                "the component {} cannot be nested in {:?}",
                self.inner.component_name(),
                parent_types
            ));
        }

        let expansion = self
            .inner
            .expand()
            .map_err(|err| format!("failed to expand transform '{}': {}", key, err))?;

        let mut ptypes = parent_types.clone();
        ptypes.insert(self.inner.component_name());

        if let Some((expanded, expand_type)) = expansion {
            let mut children = Vec::new();
            let mut inputs = self.inputs.clone();

            for (name, content) in expanded {
                let full_name = key.join(name);

                let child = TransformOuter {
                    inputs,
                    inner: content,
                };
                child.expand(full_name.clone(), &ptypes, transforms, expansions)?;
                children.push(full_name.clone());

                inputs = match expand_type {
                    ExpandType::Parallel { .. } => self.inputs.clone(),
                    ExpandType::Serial { .. } => vec![full_name.to_string()],
                }
            }

            if matches!(expand_type, ExpandType::Parallel { aggregates: true }) {
                transforms.insert(
                    key.clone(),
                    TransformOuter {
                        inputs: children.iter().map(ToString::to_string).collect(),
                        inner: Box::new(Noop),
                    },
                );
                children.push(key.clone());
            } else if matches!(expand_type, ExpandType::Serial { alias: true }) {
                transforms.insert(
                    key.clone(),
                    TransformOuter {
                        inputs,
                        inner: Box::new(Noop),
                    },
                );
                children.push(key.clone());
            }

            expansions.insert(key.clone(), children);
        } else {
            transforms.insert(key, self);
        }
        Ok(())
    }
}
