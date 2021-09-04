/// Expose metrics from /proc/net/stat
///
/// https://github.com/prometheus/node_exporter/pull/1771

use crate::event::Metric;

pub async fn gather() -> Result<Vec<Metric>, ()> {
    todo!()
}