#![allow(clippy::print_stdout)]

use std::convert::Infallible;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // For every connection, we must make a `Service` to handle all
    // incoming HTTP requests on said connection.
    let make_svc = make_service_fn(|_conn| {
        // This is the `Service` that will handle the connection.
        // `service_fn` is a helper to convert a function that
        // returns a Response into a `Service`.
        async { Ok::<_, Infallible>(service_fn(handle)) }
    });

    let addr = ([127, 0, 0, 1], 3000).into();

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}

async fn handle(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from(
        r#"component_sent_events_total 12717027
component_received_event_bytes_total 20069537574
buffer_sent_events_total{stage="0"} 10505
buffer_received_events_total{stage="0"} 10505
buffer_received_bytes_total{stage="0"} 0
component_sent_event_bytes_total 28963406318
buffer_sent_bytes_total{stage="0"} 2675778
component_received_events_total 9531490
utilization 0.000056744301474640185
buffer_max_event_size{stage="0"} 512
process_max_fds 1000000
process_open_fds 143
process_cpu_seconds_total 754.29
process_start_time_seconds 6046040.28
process_virtual_memory_bytes 156303360
process_resident_memory_bytes 61292544
process_threads 62
kafka_requests_total 1842482
kafka_requests_bytes_total 184311118
events_in_total 3177105
component_sent_event_bytes_total{output="_default"} 5967539472
kafka_consumed_messages_total 3175756
component_sent_events_total{output="_default"} 3177105
component_discarded_events_total 8564
component_errors_total{error="invalid_json"} 8564
kafka_responses_bytes_total 3342090706
metricalize_failed_total 8564
events_out_total{output="_default"} 3177105
kafka_responses_total 1842462
processed_bytes_total 2215461598
kafka_consumed_messages_bytes_total 2215756667
kafka_queue_messages_bytes 0
kafka_queue_messages 0
"#,
    )))
}
