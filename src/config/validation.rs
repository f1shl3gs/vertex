use super::builder::Builder;
use crate::config::{DataType, Resource};
use std::collections::HashMap;

pub fn warnings(builder: &Builder) -> Vec<String> {
    let mut warnings = vec![];
    let source_names = builder.sources.keys().map(|name| ("source", name.clone()));
    let transform_names = builder
        .transforms
        .keys()
        .map(|name| ("transform", name.clone()));

    for (input_type, name) in transform_names.chain(source_names) {
        if !builder
            .transforms
            .iter()
            .any(|(_, transform)| transform.inputs.contains(&name))
            && !builder
                .sinks
                .iter()
                .any(|(_, sink)| sink.inputs.contains(&name))
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

pub fn check_shape(builder: &Builder) -> Result<(), Vec<String>> {
    let mut errors = vec![];

    if builder.sources.is_empty() {
        errors.push("No sources defined in the config".to_owned());
    }

    if builder.sinks.is_empty() {
        errors.push("No sinks defined in the config".to_owned());
    }

    // helper for below
    fn tagged<'a>(
        tag: &'static str,
        iter: impl Iterator<Item = &'a String>,
    ) -> impl Iterator<Item = (&'static str, &'a String)> {
        iter.map(move |x| (tag, x))
    }

    // Check for non-unique names across sources, sinks, and transforms
    let mut ids = HashMap::<String, Vec<&'static str>>::new();
    for (ctype, id) in tagged("source", builder.sources.keys())
        .chain(tagged("transform", builder.transforms.keys()))
        .chain(tagged("sink", builder.sinks.keys()))
    {
        let uses = ids.entry(id.clone()).or_default();
        uses.push(ctype);
    }

    for (id, uses) in ids.into_iter().filter(|(_id, uses)| uses.len() > 1) {
        errors.push(format!(
            "more than one component with name \"{}\", uses ({})",
            id,
            uses.join(", "),
        ));
    }

    // Warnings and errors
    let sink_inputs = builder
        .sinks
        .iter()
        .map(|(id, sink)| ("sink", id.clone(), sink.inputs.clone()));
    let transform_inputs = builder
        .transforms
        .iter()
        .map(|(id, transform)| ("transform", id.clone(), transform.inputs.clone()));
    for (output_type, id, inputs) in sink_inputs.chain(transform_inputs) {
        if inputs.is_empty() {
            errors.push(format!(
                "{} \"{}\" has no inputs",
                capitalize(output_type),
                id
            ));
        }

        for input in inputs {
            if !builder.sources.contains_key(&input) && !builder.transforms.contains_key(&input) {
                errors.push(format!(
                    "Input \"{}\" for {} \"{}\" doesn't exists",
                    input, output_type, id,
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn typecheck(builder: &Builder) -> Result<(), Vec<String>> {
    Graph::from(builder).typecheck()
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

#[derive(Debug, Clone)]
enum Node {
    Source {
        ty: DataType,
    },
    Transform {
        input_type: DataType,
        output_type: DataType,
        inputs: Vec<String>,
    },
    Sink {
        ty: DataType,
        inputs: Vec<String>,
    },
}

#[derive(Default)]
struct Graph {
    nodes: HashMap<String, Node>,
}

impl Graph {
    fn add_source<I: Into<String>>(&mut self, id: I, ty: DataType) {
        self.nodes.insert(id.into(), Node::Source { ty });
    }

    fn add_transform<I: Into<String>>(
        &mut self,
        id: I,
        input_type: DataType,
        output_type: DataType,
        inputs: Vec<impl Into<String>>,
    ) {
        let inputs = self.clean_inputs(inputs);

        self.nodes.insert(
            id.into(),
            Node::Transform {
                input_type,
                output_type,
                inputs,
            },
        );
    }

    fn add_sink<I: Into<String>>(&mut self, id: I, ty: DataType, inputs: Vec<impl Into<String>>) {
        let inputs = self.clean_inputs(inputs);

        self.nodes.insert(id.into(), Node::Sink { ty, inputs });
    }

    fn paths(&self) -> Result<Vec<Vec<String>>, Vec<String>> {
        let mut errors = Vec::new();

        let nodes = self
            .nodes
            .iter()
            .filter_map(|(name, node)| match node {
                Node::Sink { .. } => Some(name),
                _ => None,
            })
            .flat_map(|node| {
                paths_rec(&self.nodes, node, Vec::new()).unwrap_or_else(|err| {
                    errors.push(err);
                    Vec::new()
                })
            })
            .collect();

        if errors.is_empty() {
            Ok(nodes)
        } else {
            errors.sort();
            errors.dedup();
            Err(errors)
        }
    }

    fn clean_inputs(&self, inputs: Vec<impl Into<String>>) -> Vec<String> {
        inputs.into_iter().map(Into::into).collect()
    }

    fn typecheck(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for path in self.paths()? {
            for pair in path.windows(2) {
                let (x, y) = (&pair[0], &pair[1]);
                if self.nodes.get(x).is_none() || self.nodes.get(y).is_none() {
                    continue;
                }

                match (self.nodes[x].clone(), self.nodes[y].clone()) {
                    (Node::Source { ty: ty1 }, Node::Sink { ty: ty2, .. })
                    | (
                        Node::Source { ty: ty1 },
                        Node::Transform {
                            input_type: ty2, ..
                        },
                    )
                    | (
                        Node::Transform {
                            output_type: ty1, ..
                        },
                        Node::Transform {
                            input_type: ty2, ..
                        },
                    )
                    | (
                        Node::Transform {
                            output_type: ty1, ..
                        },
                        Node::Sink { ty: ty2, .. },
                    ) => {
                        if ty1 != ty2 && ty1 != DataType::Any && ty2 != DataType::Any {
                            errors.push(format!(
                                "Data type mismatch between {} ({:?}) and {} ({:?})",
                                x, ty1, y, ty2
                            ));
                        }
                    }

                    (Node::Sink { .. }, _) | (_, Node::Source { .. }) => unreachable!(),
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            errors.sort();
            errors.dedup();
            Err(errors)
        }
    }
}

impl From<&Builder> for Graph {
    fn from(builder: &Builder) -> Self {
        let mut graph = Graph::default();

        // TODO: validate that node names are unique across sources/transforms/sinks?
        for (id, config) in builder.sources.iter() {
            graph.add_source(id.clone(), config.inner.output_type());
        }

        for (id, config) in builder.transforms.iter() {
            graph.add_transform(
                id.clone(),
                config.inner.input_type(),
                config.inner.output_type(),
                config.inputs.clone(),
            );
        }

        for (id, config) in builder.sinks.iter() {
            graph.add_sink(id.clone(), config.inner.input_type(), config.inputs.clone());
        }

        graph
    }
}

fn paths_rec(
    nodes: &HashMap<String, Node>,
    node: &str,
    mut path: Vec<String>,
) -> Result<Vec<Vec<String>>, String> {
    if let Some(i) = path.iter().position(|p| p == node) {
        let mut segment = path.split_off(i);

        segment.push(node.into());
        // I think this is maybe easier to grok from source -> sink,
        // but i'm not married to either
        segment.reverse();

        return Err(format!(
            "Cyclic dependency detected in the chain [ {} ]",
            segment
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
                .join(" -> ")
        ));
    }

    path.push(node.to_owned());
    match nodes.get(node) {
        Some(Node::Source { .. }) | None => {
            path.reverse();
            Ok(vec![path])
        }

        Some(Node::Transform { inputs, .. }) | Some(Node::Sink { inputs, .. }) => {
            let mut paths = Vec::new();

            for input in inputs {
                match paths_rec(nodes, input, path.clone()) {
                    Ok(mut p) => paths.append(&mut p),
                    Err(err) => {
                        return Err(err);
                    }
                }
            }

            Ok(paths)
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut s = s.to_owned();
    if let Some(r) = s.get_mut(0..1) {
        r.make_ascii_uppercase();
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_detects_cycles() {
        let mut graph = Graph::default();

        graph.add_source("in", DataType::Log);
        graph.add_transform("one", DataType::Log, DataType::Log, vec!["in", "three"]);
        graph.add_transform("two", DataType::Log, DataType::Log, vec!["one"]);
        graph.add_transform("three", DataType::Log, DataType::Log, vec!["two"]);
        graph.add_sink("out", DataType::Log, vec!["three"]);

        assert_eq!(
            graph.paths(),
            Err(vec![
                "Cyclic dependency detected in the chain [ three -> one -> two -> three ]".into()
            ])
        );

        let mut graph = Graph::default();
        graph.add_source("in", DataType::Log);
        graph.add_transform("one", DataType::Log, DataType::Log, vec!["in", "three"]);
        graph.add_transform("two", DataType::Log, DataType::Log, vec!["one"]);
        graph.add_transform("three", DataType::Log, DataType::Log, vec!["two"]);
        graph.add_sink("out", DataType::Log, vec!["two"]);

        assert_eq!(
            graph.paths(),
            Err(vec![
                "Cyclic dependency detected in the chain [ two -> three -> one -> two ]".into()
            ])
        );
        assert_eq!(
            graph.typecheck(),
            Err(vec![
                "Cyclic dependency detected in the chain [ two -> three -> one -> two ]".into()
            ]),
        );

        let mut graph = Graph::default();
        graph.add_source("in", DataType::Log);
        graph.add_transform("in", DataType::Log, DataType::Log, vec!["in"]);
        graph.add_sink("out", DataType::Log, vec!["in"]);

        // This isn't really a cyclic dependency but let me have this one.
        assert_eq!(
            Err(vec![
                "Cyclic dependency detected in the chain [ in -> in ]".into()
            ]),
            graph.paths()
        );
    }

    #[test]
    fn paths_doesnt_detect_noncycles() {
        let mut graph = Graph::default();

        graph.add_source("in", DataType::Log);
        graph.add_transform("one", DataType::Log, DataType::Log, vec!["in"]);
        graph.add_transform("two", DataType::Log, DataType::Log, vec!["in"]);
        graph.add_transform("three", DataType::Log, DataType::Log, vec!["one", "two"]);
        graph.add_sink("out", DataType::Log, vec!["three"]);

        graph.paths().unwrap();
    }

    #[test]
    fn detects_type_mismatches() {
        let mut graph = Graph::default();
        graph.add_source("in", DataType::Log);
        graph.add_sink("out", DataType::Metric, vec!["in"]);

        assert_eq!(
            Err(vec![
                "Data type mismatch between in (Log) and out (Metric)".into()
            ]),
            graph.typecheck()
        );
    }

    #[test]
    fn allows_log_or_metric_into_any() {
        let mut graph = Graph::default();
        graph.add_source("log_source", DataType::Log);
        graph.add_source("metric_source", DataType::Metric);
        graph.add_sink(
            "any_sink",
            DataType::Any,
            vec!["log_source", "metric_source"],
        );

        assert_eq!(Ok(()), graph.typecheck());
    }

    #[test]
    fn allows_any_into_log_or_metric() {
        let mut graph = Graph::default();
        graph.add_source("any_source", DataType::Any);
        graph.add_transform(
            "log_to_any",
            DataType::Log,
            DataType::Any,
            vec!["any_source"],
        );
        graph.add_transform(
            "any_to_log",
            DataType::Any,
            DataType::Log,
            vec!["any_source"],
        );
        graph.add_sink(
            "log_sink",
            DataType::Log,
            vec!["any_source", "log_to_any", "any_to_log"],
        );
        graph.add_sink(
            "metric_sink",
            DataType::Metric,
            vec!["any_source", "log_to_any"],
        );

        assert_eq!(graph.typecheck(), Ok(()));
    }

    #[test]
    fn allows_both_directions_for_metrics() {
        let mut graph = Graph::default();
        graph.add_source("log_source", DataType::Log);
        graph.add_source("metric_source", DataType::Metric);
        graph.add_transform(
            "log_to_log",
            DataType::Log,
            DataType::Log,
            vec!["log_source"],
        );
        graph.add_transform(
            "metric_to_metric",
            DataType::Metric,
            DataType::Metric,
            vec!["metric_source"],
        );
        graph.add_transform(
            "any_to_any",
            DataType::Any,
            DataType::Any,
            vec!["log_to_log", "metric_to_metric"],
        );
        graph.add_transform(
            "any_to_log",
            DataType::Any,
            DataType::Log,
            vec!["any_to_any"],
        );
        graph.add_transform(
            "any_to_metric",
            DataType::Any,
            DataType::Metric,
            vec!["any_to_any"],
        );
        graph.add_sink("log_sink", DataType::Log, vec!["any_to_log"]);
        graph.add_sink("metric_sink", DataType::Metric, vec!["any_to_metric"]);

        assert_eq!(Ok(()), graph.typecheck());
    }
}
