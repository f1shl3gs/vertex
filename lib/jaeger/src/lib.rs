pub mod agent;
mod thrift;
mod translate;
mod transport;

pub use crate::thrift::jaeger::{Batch, Log, Process, Span, SpanRef, SpanRefType, Tag, TagType};

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/jaeger.api_v2.rs"));

    // re-export
    pub use collector_service_server::CollectorService;

    use event::trace::SpanId;

    impl Span {
        pub fn parent_span_id(&self) -> SpanId {
            match self.references.iter().find(|reference| {
                reference.trace_id == self.trace_id
                    && reference.ref_type == SpanRefType::ChildOf as i32
            }) {
                Some(span_ref) => {
                    let mut span_id_bytes = [0u8; 8];
                    span_id_bytes.clone_from_slice(span_ref.span_id.as_slice());

                    SpanId::from_bytes(span_id_bytes)
                }
                None => SpanId::INVALID,
            }
        }
    }
}
