use std::collections::HashMap;

use super::builder::Builder;
use super::{ComponentKey, Config, OutputId, Resource};

pub fn warnings(config: &Config) -> Vec<String> {
    let mut warnings = vec![];

    let source_names = config.sources.keys().map(|name| ("source", name.clone()));
    let transform_names = config
        .transforms
        .keys()
        .map(|name| ("transform", name.clone()));

    // TODO: maybe warn about no consumers for named outputs as well?
    for (input_type, name) in transform_names.chain(source_names) {
        let id = OutputId::from(&name);
        if !config
            .transforms
            .iter()
            .any(|(_, transform)| transform.inputs.contains(&id))
            && !config
                .sinks
                .iter()
                .any(|(_, sink)| sink.inputs.contains(&id))
        {
            warnings.push(format!(
                "{} \"{}\" has no consumers",
                capitalize(input_type),
                name
            ));
        }
    }

    warnings
}

pub fn check_names<'a, I: Iterator<Item = &'a ComponentKey>>(names: I) -> Result<(), Vec<String>> {
    let errors: Vec<_> = names
        .filter(|component_key| component_key.id().contains('.'))
        .map(|component_key| {
            format!(
                "Component name \"{}\" should not contain a \".\"",
                component_key.id()
            )
        })
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn check_shape(builder: &Builder) -> Result<(), Vec<String>> {
    let mut errors = vec![];

    if builder.sources.is_empty() {
        errors.push("No sources defined in the config".to_owned());
    }

    if builder.sinks.is_empty() {
        errors.push("No sinks defined in the config".to_owned());
    }

    // Helper for below
    fn tagged<'a>(
        tag: &'static str,
        iter: impl Iterator<Item = &'a ComponentKey>,
    ) -> impl Iterator<Item = (&'static str, &'a ComponentKey)> {
        iter.map(move |x| (tag, x))
    }

    // Check for non-unique names across sources, sinks, and transforms
    let mut used_keys = HashMap::<&ComponentKey, Vec<&'static str>>::new();
    for (ctype, id) in tagged("source", builder.sources.keys())
        .chain(tagged("transform", builder.transforms.keys()))
        .chain(tagged("sink", builder.sinks.keys()))
    {
        let uses = used_keys.entry(id).or_default();
        uses.push(ctype);
    }

    for (id, uses) in used_keys.into_iter().filter(|(_id, uses)| uses.len() > 1) {
        errors.push(format!(
            "More than one component with name \"{}\" ({})",
            id,
            uses.join(", ")
        ));
    }

    // Warnings and errors
    let sink_inputs = builder
        .sinks
        .iter()
        .map(|(key, sink)| ("sink", key.clone(), sink.inputs.clone()));
    let transform_inputs = builder
        .transforms
        .iter()
        .map(|(key, transform)| ("transform", key.clone(), transform.inputs.clone()));
    for (output_type, key, inputs) in sink_inputs.chain(transform_inputs) {
        if inputs.is_empty() {
            errors.push(format!(
                "{} \"{}\" has no inputs",
                capitalize(output_type),
                key
            ));
        }

        let mut frequencies = HashMap::new();
        for input in inputs {
            let entry = frequencies.entry(input.clone()).or_insert(0usize);
            *entry += 1;
        }

        for (dup, count) in frequencies.into_iter().filter(|(_name, count)| *count > 1) {
            errors.push(format!(
                "{} \"{}\" has input \"{}\" duplicated {} times",
                capitalize(output_type),
                key,
                dup,
                count,
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn check_resources(builder: &Builder) -> Result<(), Vec<String>> {
    let source_resources = builder
        .sources
        .iter()
        .map(|(id, config)| (id, config.resources()));
    let sink_resources = builder
        .sinks
        .iter()
        .map(|(id, config)| (id, config.resources(id)));
    let extension_resources = builder
        .extensions
        .iter()
        .map(|(id, config)| (id, config.resources()));
    let conflicting_components = Resource::conflicts(
        source_resources
            .chain(sink_resources)
            .chain(extension_resources),
    );

    if conflicting_components.is_empty() {
        Ok(())
    } else {
        Err(conflicting_components
            .into_iter()
            .map(|(resource, components)| {
                format!(
                    "Resource \"{}\" is claimed by multiple components: {:?}",
                    resource, components,
                )
            })
            .collect())
    }
}

/// Check that provide + topology config aren't present in the same builder, which is an error
pub fn check_provider(builder: &Builder) -> Result<(), Vec<String>> {
    if builder.provider.is_some()
        && (!builder.extensions.is_empty()
            || !builder.sources.is_empty()
            || !builder.transforms.is_empty()
            || !builder.sinks.is_empty())
    {
        Err(vec![
            "No extensions/sources/transforms/sinks are allowed if provider config is present"
                .to_string(),
        ])
    } else {
        Ok(())
    }
}

fn capitalize(s: &str) -> String {
    let mut s = s.to_owned();
    if let Some(r) = s.get_mut(0..1) {
        r.make_ascii_uppercase();
    }

    s
}
