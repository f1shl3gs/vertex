use event::{Metric, tags};
use tokio::runtime::Handle;

pub fn metrics() -> Vec<Metric> {
    let stats = Handle::current().metrics();

    let mut metrics = vec![
        Metric::gauge(
            "tokio_num_workers",
            "The number of worker threads used by the runtime",
            stats.num_workers(),
        ),
        Metric::gauge(
            "tokio_alive_tasks",
            "The current number of alive tasks in the runtime",
            stats.num_alive_tasks(),
        ),
        Metric::gauge(
            "tokio_global_queue_depth",
            "The number of currently scheduled in the runtime's global queue",
            stats.global_queue_depth(),
        ),
    ];

    for worker in 0..stats.num_workers() {
        let busy = stats.worker_total_busy_duration(worker);
        let parks = stats.worker_park_count(worker);
        let unparks = stats.worker_park_unpark_count(worker);

        metrics.extend([
            Metric::sum_with_tags(
                "tokio_worker_busy_seconds",
                "The amount of time the worker thread has been busy",
                busy,
                tags!(
                    "worker" => worker,
                ),
            ),
            Metric::sum_with_tags(
                "tokio_worker_park_total",
                "The total number of times the worker thread has been parked",
                parks,
                tags!(
                    "worker" => worker,
                ),
            ),
            Metric::sum_with_tags(
                "tokio_worker_unpark_total",
                "The total number of times the worker thread has been parked and unparked",
                unparks,
                tags!(
                    "worker" => worker,
                ),
            ),
        ]);
    }

    metrics
}
