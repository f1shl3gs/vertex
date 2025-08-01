use std::path::Path;

use glob;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use super::extension::ExtensionOuter;
use super::global::default_data_dir;
use super::graph::Graph;
use super::provider::ProviderConfig;
use super::sink::SinkOuter;
use super::source::SourceOuter;
use super::transform::{TransformOuter, get_transform_output_ids};
use super::validation;
use super::{ComponentKey, Config, GlobalOptions, HealthcheckOptions, OutputId};
use super::{SinkConfig, SourceConfig, TransformConfig};

/// A complete Vertex configuration.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Builder {
    #[serde(default, flatten)]
    pub global: GlobalOptions,

    /// Optional configuration provider to use.
    pub provider: Option<Box<dyn ProviderConfig>>,

    #[serde(default)]
    pub healthcheck: HealthcheckOptions,

    /// All configured extensions
    #[serde(default)]
    pub extensions: IndexMap<ComponentKey, ExtensionOuter>,

    /// All configured sources.
    #[serde(default)]
    pub sources: IndexMap<ComponentKey, SourceOuter>,

    /// All configured transforms.
    #[serde(default)]
    pub transforms: IndexMap<ComponentKey, TransformOuter<String>>,

    /// All configured sinks
    #[serde(default)]
    pub sinks: IndexMap<ComponentKey, SinkOuter<String>>,
}

impl Builder {
    pub fn build(self) -> Result<Config, Vec<String>> {
        let (config, warnings) = self.build_with_warnings()?;

        for warning in warnings {
            warn!(message = warning);
        }

        Ok(config)
    }

    pub fn build_with_warnings(self) -> Result<(Config, Vec<String>), Vec<String>> {
        compile(self)
    }

    pub fn set_data_dir(&mut self, path: &Path) {
        self.global.data_dir = Some(path.to_owned())
    }

    pub fn append(&mut self, with: Self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        self.provider = with.provider;

        if self.global.data_dir.is_none() || self.global.data_dir == default_data_dir() {
            self.global.data_dir = with.global.data_dir;
        } else if with.global.data_dir != default_data_dir()
            && self.global.data_dir != with.global.data_dir
        {
            // if two configs both set 'data_dir' and have conflicting values,
            // we consider this an error.
            errors.push("conflicting values for 'data_dir' found".to_owned());
        }

        // If the user has multiple config files, we must *merge* log schemas
        // until we meet a conflict, then we are allowed to error
        if let Err(merge_errs) = self.global.log_schema.merge(&with.global.log_schema) {
            errors.extend(merge_errs)
        }

        self.healthcheck.merge(with.healthcheck);

        with.sources.keys().for_each(|k| {
            if self.sources.contains_key(k) {
                errors.push(format!("duplicate source name found: {k}"));
            }
        });

        with.transforms.keys().for_each(|k| {
            if self.transforms.contains_key(k) {
                errors.push(format!("duplicate transform name found: {k}"))
            }
        });

        with.sinks.keys().for_each(|k| {
            if self.sinks.contains_key(k) {
                errors.push(format!("duplicate sink name found: {k}"))
            }
        });

        with.sinks.keys().for_each(|k| {
            if self.extensions.contains_key(k) {
                errors.push(format!("duplicate extension name found: {k}"))
            }
        });

        if !errors.is_empty() {
            return Err(errors);
        }

        self.sources.extend(with.sources);
        self.transforms.extend(with.transforms);
        self.sinks.extend(with.sinks);
        self.extensions.extend(with.extensions);

        Ok(())
    }

    pub fn add_source<S: SourceConfig + 'static, T: Into<String>>(&mut self, id: T, source: S) {
        self.sources
            .insert(ComponentKey::from(id.into()), SourceOuter::new(source));
    }

    pub fn add_sink<S: SinkConfig + 'static, T: Into<String>>(
        &mut self,
        id: T,
        inputs: &[&str],
        sink: S,
    ) {
        let inputs = inputs
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>();
        let sink = SinkOuter::new(inputs, Box::new(sink));
        self.add_sink_outer(id, sink);
    }

    pub fn add_sink_outer(&mut self, id: impl Into<String>, sink: SinkOuter<String>) {
        self.sinks.insert(ComponentKey::from(id.into()), sink);
    }

    pub fn add_transform<T: TransformConfig + 'static, S: Into<String>>(
        &mut self,
        id: S,
        inputs: &[&str],
        transform: T,
    ) {
        let inputs = inputs
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>();
        let transform = TransformOuter {
            inner: Box::new(transform),
            inputs,
        };

        self.transforms
            .insert(ComponentKey::from(id.into()), transform);
    }

    #[cfg(test)]
    pub fn add_extension<T: super::ExtensionConfig + 'static, S: Into<String>>(
        &mut self,
        name: S,
        extension: T,
    ) {
        self.extensions.insert(
            ComponentKey::from(name.into()),
            ExtensionOuter {
                proxy: Default::default(),
                inner: Box::new(extension),
            },
        );
    }
}

pub fn compile(mut builder: Builder) -> Result<(Config, Vec<String>), Vec<String>> {
    let mut errors = Vec::new();

    // component names should not have dots in the configuration file
    // but components can expand(like route) to have components with
    // a dot so this check should be done before expanding components
    if let Err(errs) = validation::check_names(
        builder
            .transforms
            .keys()
            .chain(builder.sources.keys())
            .chain(builder.sinks.keys()),
    ) {
        errors.extend(errs);
    }

    expand_globs(&mut builder);

    if let Err(type_errors) = validation::check_shape(&builder) {
        errors.extend(type_errors);
    }

    if let Err(type_errors) = validation::check_resources(&builder) {
        errors.extend(type_errors);
    }

    if let Err(output_errors) = validation::check_outputs(&builder) {
        errors.extend(output_errors);
    }

    let Builder {
        sources,
        transforms,
        sinks,
        global,
        healthcheck,
        extensions,
        ..
    } = builder;

    let graph = match Graph::new(&sources, &transforms, &sinks) {
        Ok(graph) => graph,
        Err(err) => {
            errors.extend(err);
            return Err(errors);
        }
    };

    if let Err(errs) = graph.typecheck() {
        errors.extend(errs);
    }

    if let Err(err) = graph.check_for_cycles() {
        errors.push(err);
    }

    // Inputs are resolved from string into OutputIds as part of graph construction, so update them
    // here before adding to the final config (the types require this).
    let sinks = sinks
        .into_iter()
        .map(|(key, sink)| {
            let inputs = graph.inputs_for(&key);
            (key, sink.with_inputs(inputs))
        })
        .collect();

    let transforms = transforms
        .into_iter()
        .map(|(key, transform)| {
            let inputs = graph.inputs_for(&key);
            (key, transform.with_inputs(inputs))
        })
        .collect();

    if errors.is_empty() {
        let mut config = Config {
            global,
            healthcheck,
            sources,
            sinks,
            transforms,
            extensions,
        };

        config.propagate_acknowledgements()?;

        let warnings = validation::warnings(&config);

        Ok((config, warnings))
    } else {
        Err(errors)
    }
}

/// Expand globs in input lists
fn expand_globs(builder: &mut Builder) {
    let candidates = builder
        .sources
        .iter()
        .flat_map(|(key, source)| {
            source.inner.outputs().into_iter().map(|output| OutputId {
                component: key.clone(),
                port: output.port,
            })
        })
        .chain(builder.transforms.iter().flat_map(|(key, transform)| {
            get_transform_output_ids(transform.inner.as_ref(), key.clone())
        }))
        .map(|output_id| output_id.to_string())
        .collect::<IndexSet<String>>();

    for (id, transform) in builder.transforms.iter_mut() {
        expand_globs_inner(&mut transform.inputs, &id.to_string(), &candidates);
    }

    for (id, sink) in builder.sinks.iter_mut() {
        expand_globs_inner(&mut sink.inputs, &id.to_string(), &candidates);
    }
}

enum InputMatcher {
    Pattern(glob::Pattern),
    String(String),
}

impl InputMatcher {
    fn matches(&self, candidate: &str) -> bool {
        use InputMatcher::*;

        match self {
            Pattern(pattern) => pattern.matches(candidate),
            String(s) => s == candidate,
        }
    }
}

fn expand_globs_inner(inputs: &mut Vec<String>, id: &str, candidates: &IndexSet<String>) {
    let raw_inputs = std::mem::take(inputs);
    for raw_input in raw_inputs {
        let matcher = glob::Pattern::new(&raw_input)
            .map(InputMatcher::Pattern)
            .unwrap_or_else(|err| {
                warn!(
                    message = "Invalid glob pattern for input",
                    component_id = %id,
                    %err,
                );

                InputMatcher::String(raw_input.to_string())
            });
        let mut matched = false;
        for input in candidates {
            if matcher.matches(input) && input != id {
                matched = true;
                inputs.push(input.clone())
            }
        }
        // If it didn't work as a glob pattern, leave it in the inputs as-is. This lets us give
        // more accurate error messages about non-existent inputs.
        if !matched {
            inputs.push(raw_input)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DataType, Output, SinkContext, SourceContext, TransformContext};
    use async_trait::async_trait;
    use configurable::configurable_component;

    #[configurable_component(source, name = "mock")]
    struct MockSourceConfig;

    #[configurable_component(transform, name = "mock")]
    #[derive(Clone)]
    struct MockTransformConfig;

    #[configurable_component(sink, name = "mock")]
    struct MockSinkConfig;

    #[async_trait]
    #[typetag::serde(name = "mock")]
    impl SourceConfig for MockSourceConfig {
        async fn build(&self, _cx: SourceContext) -> crate::Result<crate::Source> {
            unimplemented!()
        }

        fn outputs(&self) -> Vec<Output> {
            vec![Output::logs()]
        }

        fn can_acknowledge(&self) -> bool {
            false
        }
    }

    #[async_trait]
    #[typetag::serde(name = "mock")]
    impl TransformConfig for MockTransformConfig {
        async fn build(&self, _cx: &TransformContext) -> crate::Result<crate::Transform> {
            todo!()
        }

        fn input_type(&self) -> DataType {
            DataType::All
        }

        fn outputs(&self) -> Vec<Output> {
            vec![Output::new(DataType::All)]
        }
    }

    #[async_trait]
    #[typetag::serde(name = "mock")]
    impl SinkConfig for MockSinkConfig {
        async fn build(
            &self,
            _cx: SinkContext,
        ) -> crate::Result<(crate::Sink, crate::Healthcheck)> {
            unimplemented!()
        }

        fn input_type(&self) -> DataType {
            DataType::All
        }
    }

    #[test]
    fn glob_expansion() {
        let mut builder = Builder::default();
        builder.add_source("foo1", MockSourceConfig);
        builder.add_source("foo2", MockSourceConfig);
        builder.add_source("bar", MockSourceConfig);
        builder.add_transform("foos", &["foo*"], MockTransformConfig);
        builder.add_sink("baz", &["foos*", "b*"], MockSinkConfig);
        builder.add_sink("quix", &["*oo*"], MockSinkConfig);
        builder.add_sink("quux", &["*"], MockSinkConfig);

        let config = builder.build().expect("build should succeed");

        assert_eq!(
            config
                .transforms
                .get(&ComponentKey::from("foos"))
                .map(|item| without_ports(item.inputs.clone()))
                .unwrap(),
            vec!["foo1".into(), "foo2".into()]
        );

        assert_eq!(
            config
                .sinks
                .get(&ComponentKey::from("baz"))
                .map(|item| without_ports(item.inputs.clone()))
                .unwrap(),
            vec!["foos".into(), "bar".into()]
        );

        assert_eq!(
            config
                .sinks
                .get(&ComponentKey::from("quux"))
                .map(|item| without_ports(item.inputs.clone()))
                .unwrap(),
            vec!["foo1".into(), "foo2".into(), "bar".into(), "foos".into()]
        );

        assert_eq!(
            config
                .sinks
                .get(&ComponentKey::from("quix"))
                .map(|item| without_ports(item.inputs.clone()))
                .unwrap(),
            vec!["foo1".into(), "foo2".into(), "foos".into()]
        );
    }

    fn without_ports(outputs: Vec<OutputId>) -> Vec<ComponentKey> {
        outputs
            .into_iter()
            .map(|output| {
                assert!(output.port.is_none());
                output.component
            })
            .collect()
    }
}
