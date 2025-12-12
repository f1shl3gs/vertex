use std::collections::HashMap;
use std::io::Write;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use event::LogRecord;
use finalize::{EventFinalizers, Finalizable};
use framework::sink::Compression;
use framework::sink::encoding::Encoder;
use framework::sink::request_builder::{EncodeResult, RequestBuilder};
use serde::Serialize;
use value::OwnedValuePath;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    labels: HashMap<String, String>,
    annotations: HashMap<String, String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    starts_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ends_at: Option<DateTime<Utc>>,
    // #[serde(rename = "generatorURL", skip_serializing_if = "Option::is_none")]
    // generator_url: Option<String>,
}

pub struct AlertEncoder {
    labels: HashMap<String, OwnedValuePath>,
    annotations: HashMap<String, OwnedValuePath>,
}

impl Encoder<Vec<LogRecord>> for AlertEncoder {
    fn encode(&self, logs: Vec<LogRecord>, writer: &mut dyn Write) -> std::io::Result<usize> {
        let mut alerts = Vec::with_capacity(logs.len());

        for log in logs {
            let mut labels = HashMap::with_capacity(self.labels.len());
            for (key, path) in &self.labels {
                let Some(value) = log.value().get(path) else {
                    continue;
                };
                labels.insert(key.clone(), value.to_string_lossy().to_string());
            }

            let mut annotations = HashMap::with_capacity(self.annotations.len());
            for (key, path) in &self.annotations {
                let Some(value) = log.value().get(path) else {
                    continue;
                };
                annotations.insert(key.clone(), value.to_string_lossy().to_string());
            }

            alerts.push(Alert {
                labels,
                annotations,
                starts_at: None,
                ends_at: None,
            })
        }

        let data = serde_json::to_vec(&alerts).map_err(std::io::Error::other)?;
        let len = data.len();

        writer.write_all(data.as_slice())?;

        Ok(len)
    }
}

#[derive(Clone)]
pub struct AlertsRequest {
    pub data: Bytes,
    pub finalizers: EventFinalizers,
}

impl Finalizable for AlertsRequest {
    fn take_finalizers(&mut self) -> EventFinalizers {
        std::mem::take(&mut self.finalizers)
    }
}

pub struct AlertmanagerRequestBuilder {
    encoder: AlertEncoder,
}

impl AlertmanagerRequestBuilder {
    pub fn new(
        labels: HashMap<String, OwnedValuePath>,
        annotations: HashMap<String, OwnedValuePath>,
    ) -> Self {
        Self {
            encoder: AlertEncoder {
                labels,
                annotations,
            },
        }
    }
}

impl RequestBuilder<Vec<LogRecord>> for AlertmanagerRequestBuilder {
    type Metadata = EventFinalizers;
    type Events = Vec<LogRecord>;
    type Encoder = AlertEncoder;
    type Payload = Bytes;
    type Request = AlertsRequest;
    type Error = std::io::Error;

    fn compression(&self) -> Compression {
        Compression::None
    }

    fn encoder(&self) -> &Self::Encoder {
        &self.encoder
    }

    fn split_input(&self, mut input: Vec<LogRecord>) -> (Self::Metadata, Self::Events) {
        let finalizers = input.take_finalizers();
        (finalizers, input)
    }

    fn build_request(
        &self,
        finalizers: Self::Metadata,
        payload: EncodeResult<Self::Payload>,
    ) -> Self::Request {
        AlertsRequest {
            finalizers,
            data: payload.data,
        }
    }
}
