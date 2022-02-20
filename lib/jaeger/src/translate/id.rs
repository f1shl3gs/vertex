use event::trace::TraceId;

pub(crate) fn to_trace_id(high: i64, low: i64) -> TraceId {
    let mut buf = [0u8; 16];
    let high: [u8; 8] = high.to_be_bytes();
    let low: [u8; 8] = low.to_be_bytes();

    buf[..8].clone_from_slice(&high);
    buf[8..].clone_from_slice(&low);

    TraceId::from_bytes(buf)
}

pub(crate) fn internal_trace_id_to_jaeger_trace_id(trace_id: TraceId) -> (i64, i64) {
    let bytes = trace_id.to_bytes();
    let (high, low) = bytes.split_at(8);
    let high = i64::from_be_bytes(high.try_into().unwrap());
    let low = i64::from_be_bytes(low.try_into().unwrap());

    (high, low)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_id_converts() {
        let inputs = [123u128, u64::MAX as u128 + u32::MAX as u128];

        for want in inputs {
            let trace_id = TraceId(want);
            let (high, low) = internal_trace_id_to_jaeger_trace_id(trace_id);
            let got = to_trace_id(high, low);

            assert_eq!(got.0, want);
        }
    }
}
