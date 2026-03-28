use event::Metric;

use super::{Error, Paths};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let content = std::fs::read_to_string(paths.proc().join("net/xfrm_stat"))?;

    let mut metrics = Vec::with_capacity(30);
    for line in content.lines() {
        let mut fields = line.split_ascii_whitespace();
        let Some(key) = fields.next() else { continue };
        let value = match fields.next() {
            Some(value) => value.parse::<f64>()?,
            None => return Err(Error::Malformed("xfrm_stat")),
        };

        let (name, desc) = match key {
            "XfrmInError" => (
                "node_xfrm_in_error_packets_total",
                "All errors not matched by other",
            ),
            "XfrmInBufferError" => (
                "node_xfrm_in_buffer_error_packets_total",
                "No buffer is left",
            ),
            "XfrmInHdrError" => ("node_xfrm_in_hdr_error_packets_total", "Header error"),
            "XfrmInNoStates" => (
                "node_xfrm_in_no_states_packets_total",
                "No state is found i.e. Either inbound SPI, address, or IPsec protocol at SA is wrong",
            ),
            "XfrmInStateProtoError" => (
                "node_xfrm_in_state_proto_error_packets_total",
                "Transformation protocol specific error e.g. SA key is wrong",
            ),
            "XfrmInStateModeError" => (
                "node_xfrm_in_state_mode_error_packets_total",
                "Transformation mode specific error",
            ),
            "XfrmInStateSeqError" => (
                "node_xfrm_in_state_seq_error_packets_total",
                "Sequence error i.e. Sequence number is out of window",
            ),
            "XfrmInStateExpired" => ("node_xfrm_in_state_expired", "State is expired"),
            "XfrmInStateInvalid" => (
                "node_xfrm_in_state_invalid_packets_total",
                "State is invalid",
            ),
            "XfrmInTmplMismatch" => (
                "node_xfrm_in_tmpl_mismatch_packets_total",
                "No matching template for states e.g. Inbound SAs are correct but SP rule is wrong",
            ),
            "XfrmInNoPols" => (
                "node_xfrm_in_no_pols_packets_total",
                "No policy is found for states e.g. Inbound SAs are correct but no SP is found",
            ),
            "XfrmInPolBlock" => ("node_xfrm_in_pol_block_packets_total", "Policy discards"),
            "XfrmInPolError" => ("node_xfrm_in_pol_error_packets_total", "Policy error"),
            "XfrmOutError" => (
                "node_xfrm_out_error_packets_total",
                "All errors which is not matched others",
            ),
            "XfrmInStateMismatch" => (
                "node_xfrm_in_state_mismatch_packets_total",
                "State has mismatch option e.g. UDP encapsulation type is mismatch",
            ),
            "XfrmOutBundleGenError" => (
                "node_xfrm_out_bundle_gen_error_packets_total",
                "Bundle generation error",
            ),
            "XfrmOutBundleCheckError" => (
                "node_xfrm_out_bundle_check_error_packets_total",
                "Bundle check error",
            ),
            "XfrmOutNoStates" => ("node_xfrm_out_no_states_packets_total", "No state is found"),
            "XfrmOutStateProtoError" => (
                "node_xfrm_out_state_proto_error_packets_total",
                "Transformation protocol specific error",
            ),
            "XfrmOutStateModeError" => (
                "node_xfrm_out_state_mode_error_packets_total",
                "Transformation mode specific error",
            ),
            "XfrmOutStateSeqError" => (
                "node_xfrm_out_state_seq_error_packets_total",
                "Sequence error i.e. Sequence number overflow",
            ),
            "XfrmOutStateExpired" => (
                "node_xfrm_out_state_expired_packets_total",
                "State is expired",
            ),
            "XfrmOutPolBlock" => ("node_xfrm_out_pol_block_packets_total", "Policy discards"),
            "XfrmOutPolDead" => ("node_xfrm_out_pol_dead_packets_total", "Policy is dead"),
            "XfrmOutPolError" => ("node_xfrm_out_pol_error_packets_total", "Policy error"),
            "XfrmFwdHdrError" => (
                "node_xfrm_fwd_hdr_error_packets_total",
                "Forward routing of a packet is not allowed",
            ),
            "XfrmOutStateInvalid" => (
                "node_xfrm_out_state_invalid_packets_total",
                "State is invalid, perhaps expired",
            ),
            "XfrmAcquireError" => (
                "node_xfrm_acquire_error_packets_total",
                "State hasn’t been fully acquired before use",
            ),
            _ => continue,
        };

        metrics.push(Metric::sum(name, desc, value));
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();

        for metric in metrics {
            println!("{metric}");
        }
    }
}
