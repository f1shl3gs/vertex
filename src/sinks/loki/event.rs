use internal::InternalEvent;

#[derive(Debug)]
pub struct LokiEventUnlabeled;

impl InternalEvent for LokiEventUnlabeled {
    fn emit_metrics(&self) {
        counter!("processing_errors_total", 1, "err" => "unlabeled_event");
    }
}

#[derive(Debug)]
pub struct LokiEventDropped;

impl InternalEvent for LokiEventDropped {
    fn emit_metrics(&self) {
        counter!("events_discarded_total", 1, "reason" => "out_of_order");
        counter!("processing_error_total", 1, "err" => "out_of_order");
    }
}

#[derive(Debug)]
pub struct LokiOutOfOrderEventRewrite;

impl InternalEvent for LokiOutOfOrderEventRewrite {
    fn emit_logs(&self) {
        warn!(
            message = "Received out-of-order event, rewriting timestamp",
            internal_log_rate_secs = 30
        );
    }

    fn emit_metrics(&self) {
        counter!("processing_errors_total", 1, "err" => "out_of_order");
    }
}