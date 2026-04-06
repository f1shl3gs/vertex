use event::Metric;

use super::{Error, Paths, read_into};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let path = paths.proc().join("sys/kernel/hung_task_detect_count");
    let count = read_into::<_, u64, _>(path)?;

    Ok(vec![Metric::sum(
        "node_kernel_hung_tasks_total",
        "Total number of tasks that have been detected as hung since the system booted",
        count,
    )])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();
        assert_eq!(metrics.len(), 1);
    }
}
