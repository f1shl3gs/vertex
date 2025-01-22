use std::collections::HashSet;
use std::fmt::Debug;

use async_trait::async_trait;
use configurable::NamedComponent;
use serde::{Deserialize, Serialize};

use super::{ComponentKey, DataType, GlobalOptions, Output, OutputId};

#[derive(Default)]
pub struct TransformContext {
    // This is optional because currently there are a lot of places we use `TransformContext`
    // that may not have the relevant data available (e.g. tests). In the future it'd be
    // nice to make it required somehow.
    pub key: Option<ComponentKey>,
    pub globals: GlobalOptions,
}

impl TransformContext {
    pub fn new_with_globals(globals: GlobalOptions) -> Self {
        Self { globals, key: None }
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

    /// Whether concurrency should be enabled for this transform.
    ///
    /// When enabled, this transform may be run in parallel in order to attempt to
    /// maximize throughput for this node in the topology. Transforms should generally
    /// not run concurrently unless they are compute-heavy, as there is a const/overhead
    /// associated with fanning out events to the parallel transform tasks.
    fn enable_concurrency(&self) -> bool {
        false
    }

    /// Whether this transform can be nested, given the types of transforms it would be
    /// nested within.
    ///
    /// For some transforms, they can expand themselves into a sub-topology of nested
    /// transforms. However, in order to prevent an infinite recursion of nested transforms,
    /// we may want to only allow one layer of "expansion". Additionally, there may be
    /// known issues with a transform that is nested under another specific transform
    /// interacting poorly, or incorrectly.
    ///
    /// This method allows a transform to report if it can or cannot function correctly
    /// if it is nested under transforms of a specific type, or if such nesting is
    /// fundamentally disallowed.
    fn nestable(&self, _parents: &HashSet<&'static str>) -> bool {
        true
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

/// Often we want to call outputs just to retrieve the OutputId's
/// without needing the schema definitions.
pub fn get_transform_output_ids<T: TransformConfig + ?Sized>(
    transform: &T,
    key: ComponentKey,
) -> impl Iterator<Item = OutputId> + '_ {
    transform.outputs().into_iter().map(move |output| OutputId {
        component: key.clone(),
        port: output.port,
    })
}
