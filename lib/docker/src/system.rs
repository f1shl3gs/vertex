use std::collections::HashMap;

use bytes::Bytes;
use futures::Stream;
use serde::Deserialize;

use super::{Client, Error, encode_filters};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Event {
    #[serde(rename = "Type")]
    pub typ: String,
    pub action: String,
}

#[derive(Debug, Default)]
pub struct EventsOptions<T> {
    pub since: Option<String>,
    pub until: Option<String>,
    pub filters: Option<HashMap<T, Vec<T>>>,
}

impl Client {
    pub async fn events<T: serde::Serialize>(
        &self,
        opts: EventsOptions<T>,
    ) -> Result<impl Stream<Item = Result<Bytes, Error>>, Error> {
        let mut params = Vec::new();

        if let Some(since) = opts.since {
            params.push(format!("since={since}"));
        }

        if let Some(until) = opts.until {
            params.push(format!("until={until}"));
        }

        if let Some(filters) = opts.filters
            && !filters.is_empty()
        {
            let encoded = encode_filters(&filters);
            params.push(format!("filters={}", encoded));
        }

        let uri = if params.is_empty() {
            "http://localhost/events".to_string()
        } else {
            format!("http://localhost/events?{}", params.join("&"))
        };

        self.stream(uri).await
    }
}
