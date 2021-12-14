/// Expose metrics from /proc/net/stat
///
/// https://github.com/prometheus/node_exporter/pull/1771
use event::Metric;

pub async fn gather() -> Result<Vec<Metric>, ()> {
    todo!()
}
