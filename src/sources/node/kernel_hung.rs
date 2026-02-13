use std::path::PathBuf;

use event::Metric;

use super::Error;

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let path = proc_path.join("sys/kernel/hung_task_detect_count");

    let count = std::fs::read_to_string(path)?.trim().parse::<u64>()?;

    Ok(vec![Metric::sum(
        "node_kernel_hung_task_detect_count",
        "Total number of tasks that have been detected as hung since the system booted",
        count as f64,
    )])
}
