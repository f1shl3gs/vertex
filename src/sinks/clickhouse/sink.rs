use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use codecs::encoding::Transformer;
use event::Event;
use framework::http::{HttpClient, HttpError};
use framework::sink::util::http::{HttpEventEncoder, HttpRetryLogic, HttpSink};
use framework::sink::util::retries::{RetryAction, RetryLogic};
use framework::HealthcheckError;
use http::{Request, StatusCode, Uri};
use hyper::Body;

use super::config::Config;

pub struct ClickhouseEventEncoder {
    transformer: Transformer,
}

impl HttpEventEncoder<BytesMut> for ClickhouseEventEncoder {
    fn encode_event(&mut self, mut event: Event) -> Option<BytesMut> {
        self.transformer.transform(&mut event);

        let log = event.into_log();
        let mut body = serde_json::to_vec(&log).expect("Event should be valid json");

        body.put_u8(b'\n');

        Some(BytesMut::from(body.as_slice()))
    }
}

#[async_trait]
impl HttpSink for Config {
    type Input = BytesMut;
    type Output = BytesMut;
    type Encoder = ClickhouseEventEncoder;

    fn build_encoder(&self) -> Self::Encoder {
        ClickhouseEventEncoder {
            transformer: self.encoding.clone(),
        }
    }

    async fn build_request(&self, events: Self::Output) -> framework::Result<Request<Bytes>> {
        let uri = set_uri_query(
            self.endpoint.as_str(),
            &self.database,
            &self.table,
            self.skip_unknown_fields,
            self.date_time_best_effort,
        )
        .expect("unable to encode uri");

        let mut builder = Request::post(&uri).header("Content-Type", "application/x-ndjson");

        if let Some(ce) = self.compression.content_encoding() {
            builder = builder.header("Content-Encoding", ce);
        }

        let mut request = builder.body(events.freeze()).unwrap();
        if let Some(auth) = &self.auth {
            auth.apply(&mut request);
        }

        Ok(request)
    }
}

pub async fn healthcheck(client: HttpClient, config: Config) -> crate::Result<()> {
    let uri = format!("{}/?query=SELECT%201", config.endpoint);
    let mut request = Request::get(uri).body(Body::empty()).unwrap();

    if let Some(auth) = &config.auth {
        auth.apply(&mut request);
    }

    let response = client.send(request).await?;

    match response.status() {
        StatusCode::OK => Ok(()),
        status => Err(HealthcheckError::UnexpectedStatus(status).into()),
    }
}

fn set_uri_query(
    uri: &str,
    database: &str,
    table: &str,
    skip_unknown: bool,
    date_time_best_effort: bool,
) -> crate::Result<Uri> {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair(
            "query",
            format!(
                "INSERT INTO \"{}\".\"{}\" FORMAT JSONEachRow",
                database,
                table.replace('\"', "\\\"")
            )
            .as_str(),
        )
        .finish();

    let mut uri = uri.to_string();
    if !uri.ends_with('/') {
        uri.push('/');
    }

    uri.push_str("?input_format_import_nested_json=1&");
    if skip_unknown {
        uri.push_str("input_format_skip_unknown_fields=1&");
    }
    if date_time_best_effort {
        uri.push_str("date_time_input_format=best_effort&")
    }
    uri.push_str(query.as_str());

    uri.parse::<Uri>().map_err(Into::into)
}

#[derive(Clone, Debug, Default)]
pub struct ClickhouseRetryLogic {
    inner: HttpRetryLogic,
}

impl RetryLogic for ClickhouseRetryLogic {
    type Error = HttpError;
    type Response = http::Response<Bytes>;

    fn is_retriable_error(&self, err: &Self::Error) -> bool {
        self.inner.is_retriable_error(err)
    }

    fn should_retry_resp(&self, resp: &Self::Response) -> RetryAction {
        match resp.status() {
            StatusCode::INTERNAL_SERVER_ERROR => {
                let body = resp.body();

                // Currently, ClickHouse returns 500's incorrect data and type
                // mismatch errors. This attempts to check if the body starts with
                // `Code: {code_num}` and to not retry those errors.
                //
                // Error code definitions: https://github.com/ClickHouse/ClickHouse/blob/master/dbms/src/Common/ErrorCodes.cpp
                //
                // Fix already merged: https://github.com/ClickHouse/ClickHouse/pull/6271
                if body.starts_with(b"Code: 117") {
                    RetryAction::DontRetry("incorrect data".into())
                } else if body.starts_with(b"Code: 53") {
                    RetryAction::DontRetry("type mismatch".into())
                } else {
                    RetryAction::Retry(String::from_utf8_lossy(body).to_string().into())
                }
            }
            _ => self.inner.should_retry_resp(resp),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_valid() {
        let uri = set_uri_query(
            "http://localhost:80",
            "my_database",
            "my_table",
            false,
            true,
        )
        .unwrap();
        assert_eq!(uri.to_string(), "http://localhost:80/?input_format_import_nested_json=1&date_time_input_format=best_effort&query=INSERT+INTO+%22my_database%22.%22my_table%22+FORMAT+JSONEachRow");

        let uri = set_uri_query(
            "http://localhost:80",
            "my_database",
            "my_\"table\"",
            false,
            false,
        )
        .unwrap();
        assert_eq!(uri.to_string(), "http://localhost:80/?input_format_import_nested_json=1&query=INSERT+INTO+%22my_database%22.%22my_%5C%22table%5C%22%22+FORMAT+JSONEachRow");
    }

    #[test]
    fn encode_invalid() {
        set_uri_query("localhost:80", "my_database", "my_table", false, false).unwrap_err();
    }
}
