use crate::InternalEvent;
use crate::update_counter;

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

pub struct KafkaStatisticsReceived {
    pub msg_cnt: f64,
    pub msg_size: f64,
    pub tx: u64,
    pub tx_bytes: u64,
    pub rx: u64,
    pub rx_bytes: u64,
    pub tx_msgs: u64,
    pub tx_msg_bytes: u64,
    pub rx_msgs: u64,
    pub rx_msg_bytes: u64,
}

impl InternalEvent for KafkaStatisticsReceived {
    fn emit_metrics(&self) {
        gauge!("kafka_queue_messages", self.msg_cnt);
        gauge!("kafka_queue_messages_bytes", self.msg_size);
        update_counter!("kafka_requests_total", self.tx);
        update_counter!("kafka_requests_bytes_total", self.tx_bytes);
        update_counter!("kafka_responses_total", self.rx);
        update_counter!("kafka_response_bytes_total", self.rx_bytes);
        update_counter!("kafka_produced_messages_total", self.tx_msgs);
        update_counter!("kafka_produced_messages_bytes_total", self.tx_msg_bytes);
        update_counter!("kafka_consumed_messages_total", self.rx_msgs);
        update_counter!("kafka_consumed_messages_bytes_total", self.rx_msg_bytes);
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