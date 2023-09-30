use framework::http::HttpError;
use framework::sink::util::retries::{RetryAction, RetryLogic};
use http::StatusCode;
use serde::Deserialize;

use super::service::ElasticsearchResponse;

#[derive(Debug, Deserialize)]
struct ElasticsearchErrorDetails {
    reason: String,
    #[serde(rename = "type")]
    err_type: String,
}

#[derive(Debug, Deserialize)]
struct ElasticsearchIndexResult {
    error: Option<ElasticsearchErrorDetails>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ElasticsearchResultItem {
    Index(ElasticsearchIndexResult),
    Create(ElasticsearchIndexResult),
}

impl ElasticsearchResultItem {
    fn result(self) -> ElasticsearchIndexResult {
        match self {
            Self::Index(r) => r,
            Self::Create(r) => r,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ElasticsearchResultResponse {
    items: Vec<ElasticsearchResultItem>,
}

#[derive(Clone)]
pub struct ElasticsearchRetryLogic;

impl RetryLogic for ElasticsearchRetryLogic {
    type Error = HttpError;
    type Response = ElasticsearchResponse;

    fn is_retriable_error(&self, _err: &Self::Error) -> bool {
        true
    }

    fn should_retry_resp(&self, resp: &Self::Response) -> RetryAction {
        let status = resp.http_response.status();

        match status {
            StatusCode::TOO_MANY_REQUESTS => RetryAction::Retry("too many requests".into()),
            StatusCode::NOT_IMPLEMENTED => {
                RetryAction::DontRetry("endpoint not implemented".into())
            }
            _ if status.is_server_error() => RetryAction::Retry(
                format!(
                    "{}: {}",
                    status,
                    String::from_utf8_lossy(resp.http_response.body())
                )
                .into(),
            ),
            _ if status.is_client_error() => {
                let body = String::from_utf8_lossy(resp.http_response.body());
                RetryAction::DontRetry(format!("client-side error, {}: {}", status, body).into())
            }
            _ if status.is_success() => {
                let body = String::from_utf8_lossy(resp.http_response.body());

                if body.contains(r#""errors":true"#) {
                    RetryAction::DontRetry(get_error_reason(&body).into())
                } else {
                    RetryAction::Successful
                }
            }
            _ => RetryAction::DontRetry(format!("response status: {}", status).into()),
        }
    }
}

fn get_error_reason(body: &str) -> String {
    match serde_json::from_str::<ElasticsearchResultResponse>(body) {
        Err(err) => format!(
            "some messages failed, could not parse response, err: {}",
            err
        ),
        Ok(resp) => match resp.items.into_iter().find_map(|item| item.result().error) {
            Some(err) => format!("error type: {}, reason: {}", err.err_type, err.reason),
            None => format!("error response: {}", body),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use event::EventStatus;
    use http::Response;

    #[test]
    fn error_response() {
        let json = "{\"took\":185,\"errors\":true,\"items\":[{\"index\":{\"_index\":\"test-hgw28jv10u\",\"_type\":\"log_lines\",\"_id\":\"3GhQLXEBE62DvOOUKdFH\",\"status\":400,\"error\":{\"type\":\"illegal_argument_exception\",\"reason\":\"mapper [message] of different type, current_type [long], merged_type [text]\"}}}]}";
        let resp = Response::builder()
            .status(StatusCode::OK)
            .body(Bytes::from(json))
            .unwrap();

        let logic = ElasticsearchRetryLogic;
        assert!(matches!(
            logic.should_retry_resp(&ElasticsearchResponse {
                http_response: resp,
                event_status: EventStatus::Rejected,
                batch_size: 1,
                events_byte_size: 1,
            }),
            RetryAction::DontRetry(_)
        ))
    }

    #[test]
    fn test_get_error_reason() {
        let tests = [
            ("{\"took\":185,\"errors\":true,\"items\":[{\"index\":{\"_index\":\"test-hgw28jv10u\",\"_type\":\"log_lines\",\"_id\":\"3GhQLXEBE62DvOOUKdFH\",\"status\":400,\"error\":{\"type\":\"illegal_argument_exception\",\"reason\":\"mapper [message] of different type, current_type [long], merged_type [text]\"}}}]}",
             "error type: illegal_argument_exception, reason: mapper [message] of different type, current_type [long], merged_type [text]"
            ),
            (
                "{\"took\":3,\"errors\":true,\"items\":[{\"create\":{\"_index\":\"test-hgw28jv10u\",\"_type\":\"_doc\",\"_id\":\"aBLq1HcBWD7eBWkW2nj4\",\"status\":400,\"error\":{\"type\":\"mapper_parsing_exception\",\"reason\":\"object mapping for [host] tried to parse field [host] as object, but found a concrete value\"}}}]}",
                "error type: mapper_parsing_exception, reason: object mapping for [host] tried to parse field [host] as object, but found a concrete value"
                )
        ];

        for (input, want) in tests {
            let got = get_error_reason(input);
            assert_eq!(got, want)
        }
    }
}
