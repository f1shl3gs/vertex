use event::Metric;
use tokio::runtime::Handle;

pub fn metrics() -> Vec<Metric> {
    let metrics = Handle::current().metrics();

    vec![
        Metric::gauge(
            "tokio_num_workers",
            "The number of worker threads used by the runtime",
            metrics.num_workers(),
        ),
        Metric::gauge(
            "tokio_alive_tasks",
            "The current number of alive tasks in the runtime",
            metrics.num_alive_tasks(),
        ),
    ]
}
