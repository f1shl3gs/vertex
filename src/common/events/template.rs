use internal::InternalEvent;
use crate::template::TemplateRenderingError;

pub struct TemplateRenderingFailed<'a> {
    pub field: Option<&'a str>,
    pub drop_event: bool,
    pub err: TemplateRenderingError,
}

impl<'a> InternalEvent for TemplateRenderingFailed<'a> {
    fn emit_logs(&self) {
        let mut msg = "Failed to render template".to_owned();
        if let Some(field) = self.field {
            use std::fmt::Write;
            let _ = write!(msg, " for \"{}\"", field);
        }

        if self.drop_event {
            msg.push_str("; discarding event");
        }

        msg.push('.');
        warn!(
            message = %msg,
            err = %self.err,
            internal_log_rate_secs = 30
        )
    }

    fn emit_metrics(&self) {
        counter!("processing_errors_total", 1, "err_type" => "render_error");
        if self.drop_event {
            counter!("events_discarded_total", 1);
        }
    }
}