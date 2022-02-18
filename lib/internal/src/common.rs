use metrics::counter;
use tracing::trace;

use crate::InternalEvent;

#[derive(Debug)]
pub struct EventsReceived {
    pub count: usize,
    pub byte_size: usize,
}

impl InternalEvent for EventsReceived {
    fn emit_metrics(&self) {
        counter!("component_received_events_total", self.count as u64);
        counter!("events_in_total", self.count as u64);
        counter!(
            "component_received_event_bytes_total",
            self.byte_size as u64
        );
    }
}

#[derive(Debug)]
pub struct EventsSent<'a> {
    pub count: usize,
    pub byte_size: usize,
    pub output: Option<&'a str>,
}

impl<'a> InternalEvent for EventsSent<'a> {
    fn emit_metrics(&self) {
        if self.count == 0 {
            return;
        }

        match self.output {
            Some(output) => {
                counter!("events_out_total", self.count as u64, "output" => output.to_owned());
                counter!("component_sent_events_total", self.count as u64, "output" => output.to_owned());
                counter!("component_sent_event_bytes_total", self.byte_size as u64, "output" => output.to_owned());
            }
            None => {
                counter!("events_out_total", self.count as u64);
                counter!("component_sent_events_total", self.count as u64);
                counter!("component_sent_event_bytes_total", self.byte_size as u64);
            }
        }
    }
}

#[derive(Debug)]
pub struct BytesSent<'a> {
    pub byte_size: usize,
    pub protocol: &'a str,
}

impl<'a> InternalEvent for BytesSent<'a> {
    fn emit_logs(&self) {
        trace!(
            message = "Bytes sent.",
            byte_size = %self.byte_size,
            protocol = %self.protocol
        );
    }

    fn emit_metrics(&self) {
        counter!(
            "component_sent_bytes_total",
            self.byte_size as u64,
            "protocol" => self.protocol.to_string(),
        );
    }

    fn name(&self) -> Option<&str> {
        Some("BytesSent")
    }
}

pub struct EventProcessed {
    pub byte_size: usize,
    pub component: &'static str,
}

impl InternalEvent for EventProcessed {
    fn emit_metrics(&self) {
        counter!("processed_bytes_total", self.byte_size as u64, "component" => self.component)
    }
}
