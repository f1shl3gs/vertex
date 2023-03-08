use std::collections::BTreeMap;

use metrics::{global_registry, Attributes, Observation, Reporter};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Throughput {
    pub count: u64,
    pub byte_size: u64,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct TopStats {
    pub sources: BTreeMap<String, Throughput>,
    pub transforms: BTreeMap<String, Throughput>,
    pub sinks: BTreeMap<String, Throughput>,

    #[serde(skip)]
    state: Option<String>,
}

impl Reporter for TopStats {
    fn start_metric(&mut self, name: &'static str, _description: &'static str) {
        if name == "component_sent_event_bytes_total"
            || name == "component_sent_events_total"
            || name == "component_received_event_bytes_total"
            || name == "component_received_events_total"
        {
            self.state = Some(name.to_string())
        }
    }

    fn report(&mut self, attrs: &Attributes, observation: Observation) {
        let metric = if let Some(metric) = &self.state {
            metric
        } else {
            return;
        };

        let mut component = String::new();
        let mut component_type = String::new();

        attrs.iter().for_each(|(key, value)| {
            if *key == "component" {
                component = value.to_string();
            }

            if *key == "component_type" {
                component_type = value.to_string();
            }
        });

        let value = match observation {
            Observation::Counter(v) => v,
            Observation::Gauge(v) => v as u64, // for now we don't have f64 value
            Observation::Histogram(_) => {
                return;
            }
        };

        match (metric.as_str(), component_type.as_str()) {
            ("component_sent_event_bytes_total", "source") => {
                let entry = self
                    .sources
                    .entry(component)
                    .or_insert_with(Throughput::default);
                entry.byte_size = value;
            }
            ("component_received_events_total", "sink") => {
                let entry = self
                    .sinks
                    .entry(component)
                    .or_insert_with(Throughput::default);
                entry.count = value;
            }
            ("component_received_event_bytes_total", "sink") => {
                let entry = self
                    .sinks
                    .entry(component)
                    .or_insert_with(Throughput::default);
                entry.byte_size = value;
            }
            _ => {}
        }
    }

    fn finish_metric(&mut self) {
        self.state = None
    }
}

impl TopStats {
    pub fn snapshot() -> Self {
        let mut stats = Self::default();
        let registry = global_registry();
        registry.report(&mut stats);

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use metrics::global_registry;

    #[test]
    fn get() {
        let registry = global_registry();
        let mut stats = TopStats::default();

        let counter = registry
            .register_counter("component_received_event_bytes_total", "sss")
            .recorder(&[("component", "foo"), ("component_type", "sink")]);

        counter.inc(111);

        registry.report(&mut stats);

        let bytes = stats.sinks.get("foo").unwrap();
        assert_eq!(bytes.byte_size, 111);
    }
}
