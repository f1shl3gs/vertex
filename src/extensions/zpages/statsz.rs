use std::collections::BTreeMap;

use metrics::{global_registry, Attributes, Observation, Reporter};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Point {
    pub attrs: BTreeMap<String, String>,
    pub value: f64,
}

#[derive(Debug, Serialize)]
pub struct Metric {
    pub name: String,
    pub description: String,
    pub points: Vec<Point>,
}

#[derive(Debug, Serialize)]
pub struct Statsz {
    pub metrics: Vec<Metric>,

    #[serde(skip)]
    state: Option<Metric>,
}

impl Reporter for Statsz {
    fn start_metric(&mut self, name: &'static str, description: &'static str) {
        self.state = Some(Metric {
            name: name.to_string(),
            description: description.to_string(),
            points: vec![],
        });
    }

    fn report(&mut self, attrs: &Attributes, observation: Observation) {
        let metric = if let Some(metric) = &mut self.state {
            metric
        } else {
            return;
        };

        let attrs = attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let value = match observation {
            Observation::Counter(c) => c as f64,
            Observation::Gauge(g) => g,
            Observation::Histogram(_) => {
                return;
            }
        };

        metric.points.push(Point { attrs, value })
    }

    fn finish_metric(&mut self) {
        match self.state.take() {
            None => {}
            Some(metric) => self.metrics.push(metric),
        }
    }
}

impl Statsz {
    pub fn snapshot() -> Self {
        let mut stats = Statsz {
            metrics: vec![],
            state: None,
        };

        global_registry().report(&mut stats);

        stats
    }
}
