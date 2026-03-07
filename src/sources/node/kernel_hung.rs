use event::Metric;

use super::{Error, Paths};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let path = paths.proc().join("sys/kernel/hung_task_detect_count");

    let count = std::fs::read_to_string(path)?.trim().parse::<u64>()?;

    Ok(vec![Metric::sum(
        "node_kernel_hung_task_detect_count",
        "Total number of tasks that have been detected as hung since the system booted",
        count as f64,
    )])
}
