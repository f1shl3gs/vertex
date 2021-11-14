use crate::InternalEvent;

#[derive(Debug)]
pub struct KafkaEventReceived {
    pub byte_size: usize,
}

impl InternalEvent for KafkaEventReceived {
    fn emit_metrics(&self) {
        counter!("events_in_total", 1);
        counter!("processed_bytes_total", self.byte_size as u64);
    }
}


pub struct KafkaEventFailed {}

pub struct KafkaOffsetUpdateFailed {}

impl InternalEvent for KafkaOffsetUpdateFailed {
    fn emit_logs(&self) {}

    fn emit_metrics(&self) {
        counter!("kafka_consumer_offset_updates_failed_total", 1);
    }
}