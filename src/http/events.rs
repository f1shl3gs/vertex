use std::time::Duration;
use http::{header, HeaderMap, HeaderValue, Request, Response};
use hyper::body::HttpBody;
use hyper::Error;
use metrics::{counter, histogram};
use internal::InternalEvent;

fn remove_sensitive(headers: &HeaderMap<HeaderValue>) -> HeaderMap<HeaderValue> {
    let mut headers = headers.clone();
    for name in &[
        header::AUTHORIZATION,
        header::PROXY_AUTHORIZATION,
        header::COOKIE,
        header::SET_COOKIE
    ] {
        if let Some(value) = headers.get_mut(name) {
            value.set_sensitive(true);
        }
    }

    headers
}


/// Newtype placeholder to provide a formatter for the request and response body.
struct FormatBody<'a, B>(&'a B);

impl<'a, B: HttpBody> std::fmt::Display for FormatBody<'a, B> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let size = self.0.size_hint();
        match (size.lower(), size.upper()) {
            (0, None) => write!(fmt, "[unknown]"),
            (lower, None) => write!(fmt, "[>={} bytes]", lower),

            (0, Some(0)) => write!(fmt, "[empty]"),
            (0, Some(upper)) => write!(fmt, "[<={} bytes]", upper),

            (lower, Some(upper)) if lower == upper => write!(fmt, "[{} bytes]", lower),
            (lower, Some(upper)) => write!(fmt, "[{}..={} bytes]", lower, upper),
        }
    }
}

#[derive(Debug)]
pub struct AboutToSendHttpRequest<'a, T> {
    pub request: &'a Request<T>,
}

impl<'a, T: HttpBody> InternalEvent for AboutToSendHttpRequest<'a, T> {
    fn emit_logs(&self) {
        debug!(
            message = "Sending HTTP request",
            uri = %self.request.uri(),
            method = %self.request.method(),
            version = ?self.request.version(),
            headers = ?remove_sensitive(self.request.headers()),
            body = %FormatBody(self.request.body()),
        )
    }

    fn emit_metrics(&self) {
        counter!(
            "http_client_requests_sent_total",
            1,
            "method" => self.request.method().to_string(),
        );
    }
}

#[derive(Debug)]
pub struct GotHttpResponse<'a, T> {
    pub response: &'a Response<T>,
    pub roundtrip: Duration,
}

impl<'a, T: HttpBody> InternalEvent for GotHttpResponse<'a, T> {
    fn emit_logs(&self) {
        debug!(
            message = "HTTP response received",
            status = %self.response.status(),
            version = ?self.response.version(),
            headers = ?remove_sensitive(self.response.headers()),
            body = %FormatBody(self.response.body())
        );
    }

    fn emit_metrics(&self) {
        counter!(
            "http_client_responses_total",
            1,
            "status" => self.response.status().as_u16().to_string()
        );
        histogram!(
            "http_client_rtt_seconds", self.roundtrip
        );
        histogram!(
            "http_client_response_rtt_seconds",
            self.roundtrip,
            "status" => self.response.status().as_u16().to_string()
        )
    }
}

#[derive(Debug)]
pub struct GotHttpError<'a> {
    pub error: &'a Error,
    pub roundtrip: Duration,
}

impl<'a> InternalEvent for GotHttpError<'a> {
    fn emit_logs(&self) {
        debug!(
            message = "HTTP error",
            error = %self.error
        );
    }

    fn emit_metrics(&self) {
        counter!("http_client_errors_total", 1, "error" => self.error.to_string());
        histogram!("http_client_rtt_seconds", self.roundtrip);
        histogram!("http_client_error_rtt_seconds", self.roundtrip, "error" => self.error.to_string());
    }
}