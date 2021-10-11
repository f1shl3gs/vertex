use serde::{Deserialize, Serialize};
use crate::config::{
    GlobalOptions,
    TransformOuter,
    SinkOuter,
    Config,
    SourceConfig,
    HealthcheckOptions,
    ExpandType,
    ExtensionConfig,
};
use indexmap::IndexMap;
use crate::config::provider::ProviderConfig;
use super::validation;
use glob;
use crate::config::global::default_data_dir;
#[cfg(test)]
use crate::config::{SinkConfig, TransformConfig};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Builder {
    pub global: GlobalOptions,
    #[serde(default)]
    pub sources: IndexMap<String, Box<dyn SourceConfig>>,
    #[serde(default)]
    pub transforms: IndexMap<String, TransformOuter>,
    #[serde(default)]
    pub sinks: IndexMap<String, SinkOuter>,
    #[serde(default)]
    pub extensions: IndexMap<String, Box<dyn ExtensionConfig>>,

    pub provider: Option<Box<dyn ProviderConfig>>,

    #[serde(rename = "health_checks")]
    pub health_checks: HealthcheckOptions,
}

impl Builder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn build(self) -> Result<Config, Vec<String>> {
        let (config, warnings) = self.build_with_warnings()?;

        for warning in warnings {
            warn!("{}", warning);
        }

        Ok(config)
    }

    pub fn build_with_warnings(self) -> Result<(Config, Vec<String>), Vec<String>> {
        compile(self)
    }

    pub fn append(&mut self, with: Self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        self.provider = with.provider;

        if self.global.data_dir.as_os_str().len() == 0 || self.global.data_dir == default_data_dir() {
            self.global.data_dir = with.global.data_dir;
        } else if with.global.data_dir != default_data_dir() && self.global.data_dir != with.global.data_dir {
            // if two configs both set 'data_dir' and have conflicting values,
            // we consider this an error.
            errors.push("conflicting values for 'data_dir' found".to_owned());
        }

        with.sources.keys().for_each(|k| {
            if self.sources.contains_key(k) {
                errors.push(format!("duplicate source name found: {}", k));
            }
        });

        with.transforms.keys().for_each(|k| {
            if self.transforms.contains_key(k) {
                errors.push(format!("duplicate transform name found: {}", k))
            }
        });

        with.sinks.keys().for_each(|k| {
            if self.sinks.contains_key(k) {
                errors.push(format!("duplicate sink name found: {}", k))
            }
        });

        with.sinks.keys().for_each(|k| {
            if self.extensions.contains_key(k) {
                errors.push(format!("duplicate extension name found: {}", k))
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

    #[cfg(test)]
    pub fn add_source<S: SourceConfig + 'static, T: Into<String>>(&mut self, name: T, source: S) {
        self.sources.insert(name.into(), Box::new(source));
    }

    #[cfg(test)]
    pub fn add_sink<S: SinkConfig + 'static, T: Into<String>>(
        &mut self,
        name: T,
        inputs: &[&str],
        sink: S,
    ) {
        let inputs = inputs.iter().map(|&s| s.to_owned()).collect::<Vec<_>>();
        let sink = SinkOuter::new(inputs, Box::new(sink));
        self.sinks.insert(name.into(), sink);
    }

    #[cfg(test)]
    pub fn add_transform<T: TransformConfig + 'static, S: Into<String>>(
        &mut self,
        name: S,
        inputs: &[&str],
        transform: T,
    ) {
        let inputs = inputs.iter().map(|&s| s.to_owned()).collect::<Vec<_>>();
        let transform = TransformOuter {
            inputs,
            inner: Box::new(transform),
        };

        self.transforms.insert(name.into(), transform);
    }

    #[cfg(test)]
    pub fn add_extension<T: ExtensionConfig + 'static, S: Into<String>>(
        &mut self,
        name: S,
        extension: T,
    ) {
        self.extensions.insert(name.into(), Box::new(extension));
    }
}

pub fn compile(mut builder: Builder) -> Result<(Config, Vec<String>), Vec<String>> {
    let mut errors = Vec::new();
    let expansions = expand_macros(&mut builder)?;

    expand_globs(&mut builder);

    let warnings = validation::warnings(&builder);
    if let Err(type_errors) = validation::check_shape(&builder) {
        errors.extend(type_errors);
    }

    if let Err(type_errors) = validation::typecheck(&builder) {
        errors.extend(type_errors);
    }

    if let Err(type_errors) = validation::check_resources(&builder) {
        errors.extend(type_errors);
    }

    if errors.is_empty() {
        Ok((
            Config {
                global: builder.global,
                health_checks: builder.health_checks,
                sources: builder.sources,
                sinks: builder.sinks,
                transforms: builder.transforms,
                extensions: builder.extensions,
                expansions,
            },
            warnings,
        ))
    } else {
        Err(errors)
    }
}

/// Some component configs can act like macros and expand themselves into multiple
/// replacement configs. Performs those expansions and records the relevant metadata.
pub fn expand_macros(
    builder: &mut Builder,
) -> Result<IndexMap<String, Vec<String>>, Vec<String>> {
    let mut expanded_transforms = IndexMap::new();
    let mut expansions = IndexMap::new();
    let mut errors = Vec::new();

    while let Some((k, mut t)) = builder.transforms.pop() {
        if let Some((expanded, expand_type)) = match t.inner.expand() {
            Ok(e) => e,
            Err(err) => {
                errors.push(format!("failed to expand transform '{}': {}", k, err));
                continue;
            }
        } {
            let mut children = Vec::new();
            let mut inputs = t.inputs.clone();

            for (name, child) in expanded {
                let fullname = format!("{}.{}", k, name);

                expanded_transforms.insert(
                    fullname.clone(),
                    TransformOuter {
                        inputs,
                        inner: child,
                    },
                );
                children.push(fullname.clone());
                inputs = match expand_type {
                    ExpandType::Parallel => t.inputs.clone(),
                    ExpandType::Serial => vec![fullname],
                }
            }

            expansions.insert(k.clone(), children);
        } else {
            expanded_transforms.insert(k, t);
        }
    }

    builder.transforms = expanded_transforms;

    if !errors.is_empty() {
        Err(errors)
    } else {
        Ok(expansions)
    }
}

/// Expand globs in input lists
fn expand_globs(builder: &mut Builder) {
    let candidates = builder
        .sources
        .keys()
        .chain(builder.transforms.keys())
        .cloned()
        .collect::<Vec<String>>();

    for (id, transform) in builder.transforms.iter_mut() {
        expand_globs_inner(&mut transform.inputs, id, &candidates);
    }

    for (id, sink) in builder.sinks.iter_mut() {
        expand_globs_inner(&mut sink.inputs, id, &candidates);
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

fn expand_globs_inner(inputs: &mut Vec<String>, id: &str, candidates: &[String]) {
    let raw_inputs = std::mem::take(inputs);

    for raw_input in raw_inputs {
        let matcher = glob::Pattern::new(&raw_input.to_string())
            .map(InputMatcher::Pattern)
            .unwrap_or_else(|err| {
                warn!(
                    message = "invalid glob pattern for input",
                    id,
                    %err
                );
                InputMatcher::String(raw_input.to_string())
            });

        for input in candidates {
            if matcher.matches(&input.to_string()) && input != id {
                inputs.push(input.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use async_trait::async_trait;
    use crate::config::{SourceContext, DataType, SinkContext, HealthCheck};
    use crate::sources::Source;
    use crate::transforms::Transform;
    use crate::sinks::Sink;

    #[derive(Debug, Serialize, Deserialize)]
    struct MockSourceConfig;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct MockTransformConfig;

    #[derive(Debug, Serialize, Deserialize)]
    struct MockSinkConfig;

    #[async_trait]
    #[typetag::serde(name = "mock")]
    impl SourceConfig for MockSourceConfig {
        async fn build(&self, _ctx: SourceContext) -> crate::Result<Source> {
            unimplemented!()
        }

        fn output_type(&self) -> DataType {
            DataType::Any
        }

        fn source_type(&self) -> &'static str {
            "mock"
        }
    }

    #[async_trait]
    #[typetag::serde(name = "mock")]
    impl TransformConfig for MockTransformConfig {
        async fn build(&self, _globals: &GlobalOptions) -> crate::Result<Transform> {
            unimplemented!()
        }

        fn input_type(&self) -> DataType {
            DataType::Any
        }

        fn output_type(&self) -> DataType {
            DataType::Any
        }

        fn transform_type(&self) -> &'static str {
            "mock"
        }
    }

    #[async_trait]
    #[typetag::serde(name = "mock")]
    impl SinkConfig for MockSinkConfig {
        async fn build(&self, _ctx: SinkContext) -> crate::Result<(Sink, HealthCheck)> {
            unimplemented!()
        }

        fn input_type(&self) -> DataType {
            DataType::Any
        }

        fn sink_type(&self) -> &'static str {
            "mock"
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
                .get("foos")
                .map(|item| item.inputs.clone())
                .unwrap(),
            vec!["foo1", "foo2"]
        );

        assert_eq!(
            config
                .sinks
                .get("baz")
                .map(|item| item.inputs.clone())
                .unwrap(),
            vec!["foos", "bar"]
        );

        assert_eq!(
            config
                .sinks
                .get("quux")
                .map(|item| item.inputs.clone())
                .unwrap(),
            vec![
                "foo1",
                "foo2",
                "bar",
                "foos",
            ]
        );

        assert_eq!(
            config
                .sinks
                .get("quix")
                .map(|item| item.inputs.clone())
                .unwrap(),
            vec![
                "foo1",
                "foo2",
                "foos",
            ]
        )
    }
}