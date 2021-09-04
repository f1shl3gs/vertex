/// Expose metrics from /proc/net/protocols
///
/// https://github.com/prometheus/node_exporter/pull/1921

use crate::event::Metric;

pub async fn gather() -> Result<Vec<Metric>, ()> {
    todo!()
}