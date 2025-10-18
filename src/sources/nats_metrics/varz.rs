use std::collections::BTreeMap;

use event::Metric;
use value::Value;

const EXCLUDE_KEYS: [&str; 9] = [
    "leaf",
    "trusted_operators_claim",
    "cluster_tls_timeout",
    "cluster_cluster_port",
    "cluster_auth_timeout",
    "gateway_port",
    "gateway_auth_timeout",
    "gateway_tls_timeout",
    "gateway_connect_retries",
];

const LABELS: [&str; 8] = [
    "server_id",
    "server_name",
    "version",
    "domain",
    "leader",
    "name",
    "start",
    "config_load_time",
];

pub fn object_to_metrics(prefix: &str, obj: BTreeMap<String, Value>, metrics: &mut Vec<Metric>) {
    for (key, value) in obj {
        if EXCLUDE_KEYS.contains(&key.as_str()) {
            continue;
        }

        let key = match key.strip_prefix('/') {
            None => key.as_str(),
            Some(stripped) => stripped,
        };

        match value {
            Value::Bytes(_) => {
                if !LABELS.contains(&key) {
                    continue;
                }

                metrics.push(Metric::gauge(format!("{prefix}_{key}"), "", 1));
            }
            Value::Integer(i) => metrics.push(Metric::gauge(format!("{prefix}_{key}"), "", i)),
            Value::Float(f) => metrics.push(Metric::gauge(format!("{prefix}_{key}"), "", f)),
            Value::Boolean(_) => {}
            Value::Timestamp(_) => {}
            Value::Object(obj) => {
                let prefix = format!("{prefix}_{key}");
                object_to_metrics(&prefix, obj, metrics);
            }
            Value::Array(_) => {}
            Value::Null => {}
        }
    }
}
