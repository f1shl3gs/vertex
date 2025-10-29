use std::borrow::Cow;
use std::collections::HashMap;

use indexmap::{IndexMap, IndexSet};
use regex::Captures;
use serde::{Deserialize, Serialize};

use super::Loader;
use super::graph::Graph;
use super::secret::COLLECTOR;
use crate::config::global::default_data_dir;
use crate::config::provider::ProviderConfig;
use crate::config::{
    ComponentKey, Config, ExtensionOuter, GlobalOptions, HealthcheckOptions, OutputId, Resource,
    SinkOuter, SourceOuter, TransformConfig, TransformOuter,
};
use crate::pipeline::DEFAULT_OUTPUT;

/// A complete Vertex configuration.
#[derive(Debug, Default, Deserialize, Serialize)]
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
    fn check_names(&self) -> Result<(), Vec<String>> {
        fn check<'a, I: Iterator<Item = &'a ComponentKey>>(
            names: I,
        ) -> Option<Vec<&'a ComponentKey>> {
            let invalids = names.filter(|key| key.contains(".")).collect::<Vec<_>>();

            if invalids.is_empty() {
                None
            } else {
                Some(invalids)
            }
        }

        let mut errs = Vec::new();
        if let Some(names) = check(self.extensions.keys()) {
            errs.push(format!("invalid extension names {names:?}"));
        }
        if let Some(names) = check(self.sources.keys()) {
            errs.push(format!("invalid source names {names:?}"));
        }
        if let Some(names) = check(self.transforms.keys()) {
            errs.push(format!("invalid transform names {names:?}"));
        }
        if let Some(names) = check(self.sinks.keys()) {
            errs.push(format!("invalid sink names {names:?}"));
        }

        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }

    fn check_shape(&self) -> Result<(), Vec<String>> {
        let mut errs = Vec::new();

        if self.sources.is_empty() {
            errs.push("No sources defined in the config".to_string());
        }

        if self.sinks.is_empty() {
            errs.push("No sinks defined in the config".to_string());
        }

        // helper for below
        fn tagged<'a>(
            tag: &'static str,
            iter: impl Iterator<Item = &'a ComponentKey>,
        ) -> impl Iterator<Item = (&'static str, &'a ComponentKey)> {
            iter.map(move |x| (tag, x))
        }

        // check for non-unique names across sources, transforms and sinks
        let mut used_keys = HashMap::<String, Vec<&'static str>>::new();
        for (typ, id) in tagged("source", self.sources.keys())
            .chain(tagged("transform", self.transforms.keys()))
            .chain(tagged("sink", self.sinks.keys()))
        {
            let uses = used_keys.entry(id.to_string()).or_default();
            uses.push(typ)
        }

        for (id, uses) in used_keys.into_iter().filter(|(_id, uses)| uses.len() > 1) {
            errs.push(format!(
                "More than one component with name {:?} ({})",
                id,
                uses.join(", ")
            ))
        }

        // Warnings and errors
        let sink_inputs = self
            .sinks
            .iter()
            .map(|(key, sink)| ("sink", key.clone(), sink.inputs.clone()));
        let transform_inputs = self
            .transforms
            .iter()
            .map(|(key, transform)| ("transform", key.clone(), transform.inputs.clone()));
        for (output_type, key, inputs) in sink_inputs.chain(transform_inputs) {
            if inputs.is_empty() {
                errs.push(format!("{} {:?} has no inputs", output_type, key));
            }

            let mut frequencies = HashMap::new();
            for input in inputs {
                let entry = frequencies.entry(input).or_insert(0usize);
                *entry += 1;
            }

            for (dup, count) in frequencies.into_iter().filter(|(_name, count)| *count > 1) {
                errs.push(format!(
                    "{} {:?} has input {:?} duplicated {} times",
                    output_type, key, dup, count
                ));
            }
        }

        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }

    fn check_resources(&self) -> Result<(), Vec<String>> {
        let extension_resources = self
            .extensions
            .iter()
            .map(|(key, config)| (key, config.resources()));
        let source_resources = self
            .sources
            .iter()
            .map(|(key, config)| (key, config.resources()));
        let sink_resources = self
            .sinks
            .iter()
            .map(|(key, config)| (key, config.resources(key)));

        let conflicting = Resource::conflicts(
            extension_resources
                .chain(source_resources)
                .chain(sink_resources),
        );

        if conflicting.is_empty() {
            Ok(())
        } else {
            Err(conflicting
                .into_iter()
                .map(|(resource, components)| {
                    format!(
                        "Resource {resource:?} is claimed by multiple components: {components:?}"
                    )
                })
                .collect())
        }
    }

    /// To avoid collisions between `output` metric tags, check that
    /// a component does not have a named output with the name [`DEFAULT_OUTPUT`]
    fn check_outputs(&self) -> Result<(), Vec<String>> {
        let mut errs = Vec::new();

        for (key, source) in &self.sources {
            let outputs = source.inner.outputs();

            if outputs
                .iter()
                .map(|output| output.port.as_deref().unwrap_or(""))
                .any(|name| name == DEFAULT_OUTPUT)
            {
                errs.push(format!(
                    "Source {key} cannot have a named output with reserved name: {DEFAULT_OUTPUT}",
                ));
            }
        }

        for (key, transform) in &self.transforms {
            // use the most general definition possible, since the real value
            // isn't known yet
            if get_transform_output_ids(transform.inner.as_ref(), key)
                .any(|output| matches!(output.port, Some(output) if output == DEFAULT_OUTPUT))
            {
                errs.push(format!(
                    "Transform {key} cannot have a named output with reserved name: {DEFAULT_OUTPUT}",
                ));
            }
        }

        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }

    fn expand_globs(&mut self) {
        let candidates = self
            .sources
            .iter()
            .flat_map(|(key, source)| {
                source.inner.outputs().into_iter().map(|output| OutputId {
                    component: key.clone(),
                    port: output.port,
                })
            })
            .chain(self.transforms.iter().flat_map(|(key, transform)| {
                get_transform_output_ids(transform.inner.as_ref(), key)
            }))
            .map(|id| id.to_string())
            .collect::<IndexSet<String>>();

        for (id, transform) in self.transforms.iter_mut() {
            expand_globs_inner(&mut transform.inputs, id, &candidates)
        }

        for (id, sink) in self.sinks.iter_mut() {
            expand_globs_inner(&mut sink.inputs, id, &candidates)
        }
    }

    pub fn compile(mut self) -> Result<Config, Vec<String>> {
        let mut errs = Vec::new();

        // component names should not have dots in the configuration file but
        // components can expand(like route) to have components with a dot so
        // this check should be done before expanding components
        if let Err(partial) = self.check_names() {
            errs.extend(partial)
        }

        self.expand_globs();

        if let Err(partial) = self.check_shape() {
            errs.extend(partial);
        }
        if let Err(partial) = self.check_resources() {
            errs.extend(partial);
        }
        if let Err(partial) = self.check_outputs() {
            errs.extend(partial);
        }

        let Builder {
            global,
            healthcheck,
            extensions,
            sources,
            transforms,
            sinks,
            ..
        } = self;

        let graph = match Graph::new(&sources, &transforms, &sinks) {
            Ok(graph) => graph,
            Err(partial) => {
                errs.extend(partial);
                return Err(errs);
            }
        };

        if let Err(partial) = graph.typecheck() {
            errs.extend(partial);
        }
        if let Err(partial) = graph.check_for_cycles() {
            errs.push(partial);
        }

        // Inputs are resolved from string into OutputIds as part of graph construction,
        // so update them here before adding to the final config (the types require this)
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

        if errs.is_empty() {
            let mut config = Config {
                global,
                healthcheck,
                extensions,
                sources,
                transforms,
                sinks,
            };

            config.propagate_acknowledgements()?;

            Ok(config)
        } else {
            Err(errs)
        }
    }
}

#[cfg(any(test, feature = "test-util"))]
impl Builder {
    pub fn set_data_dir(&mut self, path: &std::path::Path) {
        self.global.data_dir = Some(path.to_owned())
    }

    pub fn add_source<S: crate::config::SourceConfig + 'static, T: Into<String>>(
        &mut self,
        id: T,
        source: S,
    ) {
        self.sources
            .insert(ComponentKey::from(id.into()), SourceOuter::new(source));
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

    pub fn add_sink<S: crate::config::SinkConfig + 'static, T: Into<String>>(
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
}

fn get_transform_output_ids<T: TransformConfig + ?Sized>(
    transform: &T,
    key: &str,
) -> impl Iterator<Item = OutputId> {
    transform.outputs().into_iter().map(|output| OutputId {
        component: key.into(),
        port: output.port,
    })
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
                    message = "invalid glob pattern for input",
                    component = id,
                    %err
                );

                InputMatcher::String(raw_input.to_string())
            });

        let mut matched = false;
        for input in candidates {
            if input != id && matcher.matches(input) {
                matched = true;
                inputs.push(input.clone());
            }
        }

        // if it didn't work as a glob pattern, leave it in the inputs as-is.
        // This lets us give more accurate error messages about non-existent
        // inputs.
        if !matched {
            inputs.push(raw_input);
        }
    }
}

pub struct ConfigLoader {
    secrets: HashMap<String, HashMap<String, String>>,

    builder: Builder,
}

impl ConfigLoader {
    pub fn new(secrets: HashMap<String, HashMap<String, String>>) -> Self {
        Self {
            secrets,
            builder: Builder::default(),
        }
    }
}

impl Loader for ConfigLoader {
    type Output = Builder;
    type Item = Builder;

    fn prepare<'a>(&mut self, input: &'a str) -> Result<Cow<'a, str>, Vec<String>> {
        if self.secrets.is_empty() {
            return Ok(Cow::Borrowed(input));
        }

        let mut errs = Vec::new();
        let output = COLLECTOR.replace_all(input, |caps: &Captures<'_>| {
            caps.get(1)
                .and_then(|s| caps.get(2).map(|k| (s, k)))
                .and_then(|(s, k)| {
                    self.secrets
                        .get(s.as_str())?
                        .get(k.as_str())
                        .map(|s| s.as_str())
                })
                .unwrap_or_else(|| {
                    errs.push(format!(
                        "unable to find secret replacement for {}",
                        caps.get(0).unwrap().as_str()
                    ));
                    ""
                })
        });

        if errs.is_empty() {
            Ok(output)
        } else {
            Err(errs)
        }
    }

    fn merge(&mut self, other: Self::Item) -> Result<(), Vec<String>> {
        let mut errs = Vec::new();

        let builder = &mut self.builder;

        builder.provider = other.provider;

        if builder.global.data_dir.is_none() || builder.global.data_dir == default_data_dir() {
            builder.global.data_dir = other.global.data_dir;
        } else if other.global.data_dir != default_data_dir()
            && builder.global.data_dir != other.global.data_dir
        {
            // if two configs both set 'data_dir' and have conflicting values,
            // we consider this an error
            errs.push("conflicting values for 'data_dir' found".to_string());
        }

        // If the user has multiple config files, we must *merge* log schemas until
        // we meet a conflict, then we are allowed to error
        if let Err(partial) = builder.global.log_schema.merge(&other.global.log_schema) {
            errs.extend(partial);
        }

        builder.healthcheck.merge(other.healthcheck);

        other.extensions.keys().for_each(|key| {
            if builder.extensions.contains_key(key) {
                errs.push(format!("duplicate extension name found for {key}"));
            }
        });
        other.sources.keys().for_each(|key| {
            if builder.sources.contains_key(key) {
                errs.push(format!("duplicate source name found for {key}"));
            }
        });
        other.transforms.keys().for_each(|key| {
            if builder.transforms.contains_key(key) {
                errs.push(format!("duplicate transform name found for {key}"));
            }
        });
        other.sinks.keys().for_each(|key| {
            if builder.sinks.contains_key(key) {
                errs.push(format!("duplicate sink name found for {key}"));
            }
        });

        if errs.is_empty() {
            builder.extensions.extend(other.extensions);
            builder.sources.extend(other.sources);
            builder.transforms.extend(other.transforms);
            builder.sinks.extend(other.sinks);

            Ok(())
        } else {
            Err(errs)
        }
    }

    fn build(self) -> Result<Self::Output, Vec<String>> {
        Ok(self.builder)
    }
}
