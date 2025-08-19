use bytes::{Buf, BufMut, Bytes, BytesMut};
use chrono::{DateTime, Utc};
use configurable::Configurable;
use serde::{Deserialize, Serialize};
use tail::decode::Error as DelimitError;
use tail::multiline::Logic;

pub enum Error {
    Frame(DelimitError),

    Timestamp(chrono::format::ParseError),

    TooShort,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Frame(err) => err.fmt(f),
            Error::Timestamp(err) => err.fmt(f),
            Error::TooShort => f.write_str("The message is too short"),
        }
    }
}

#[derive(Configurable, Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stream {
    #[default]
    All,

    Stdout,

    Stderr,
}

impl Stream {
    fn matched(&self, input: &[u8]) -> bool {
        match self {
            Stream::All => true,
            Stream::Stdout => input == b"stdout",
            Stream::Stderr => input == b"stderr",
        }
    }
}

pub struct Cri {
    last: bool,
}

impl Clone for Cri {
    fn clone(&self) -> Self {
        Self { last: true }
    }
}

impl Default for Cri {
    fn default() -> Self {
        Self { last: true }
    }
}

impl Logic for Cri {
    fn is_start(&mut self, line: &[u8]) -> bool {
        let mut parts = line.splitn(4, |&b| b == b' ');

        let Some(_timestamp) = parts.next() else {
            return true;
        };

        let Some(_stream) = parts.next() else {
            return true;
        };

        let Some(tag) = parts.next() else {
            return true;
        };

        let start = self.last;

        // if last tag is `F`, then this line must be a new start line
        self.last = tag == b"F";

        start
    }

    fn merge(&self, stashed: &mut BytesMut, mut data: Bytes) {
        // strip timestamp, stream and tag
        // 2019-05-07T18:57:50.904275087+00:00 stdout P message
        if let Some(index) = data.iter().position(|c| *c == b'P' || *c == b'F') {
            // 1 for tag, 1 for space
            let cnt = (index + 1 + 1).min(data.len());
            data.advance(cnt);
        };

        stashed.put(data);
    }
}

/// Parse the merged message, and return timestamp, stream and message
///
/// ```text
/// 2019-05-07T18:57:50.904275087+00:00 stdout P message
/// ```
pub fn parse(line: Bytes, filter: &Stream) -> Result<(DateTime<Utc>, Bytes, Bytes), Error> {
    let mut parts = line.splitn(4, |&b| b == b' ');
    let Some(timestamp) = parts.next() else {
        return Err(Error::TooShort);
    };

    let Some(stream) = parts.next() else {
        return Err(Error::TooShort);
    };

    if !filter.matched(stream) {
        return Err(Error::TooShort);
    }

    let timestamp = DateTime::parse_from_rfc3339(String::from_utf8_lossy(timestamp).as_ref())
        .map(|ts| ts.to_utc())
        .map_err(Error::Timestamp)?;

    let Some(_tag) = parts.next() else {
        return Err(Error::TooShort);
    };

    let Some(msg) = parts.next() else {
        return Err(Error::TooShort);
    };

    Ok((timestamp, line.slice_ref(stream), line.slice_ref(msg)))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bytes::Bytes;
    use futures::StreamExt;
    use tail::multiline::Multiline;

    use super::*;

    #[tokio::test]
    async fn cri() {
        let input = [
            "2019-05-07T18:57:50.904275087+00:00 stdout P 1a. some ",
            "2019-05-07T18:57:51.904275088+00:00 stdout P multiline ",
            "2019-05-07T18:57:52.904275089+00:00 stdout F log",
            "2019-05-07T18:57:50.904275087+00:00 stderr P 1b. some ",
            "2019-05-07T18:57:51.904275088+00:00 stderr P multiline ",
            "2019-05-07T18:57:52.904275089+00:00 stderr F log",
            "2019-05-07T18:57:53.904275090+00:00 stdout P 2a. another ",
            "2019-05-07T18:57:54.904275091+00:00 stdout P multiline ",
            "2019-05-07T18:57:55.904275092+00:00 stdout F log",
            "2019-05-07T18:57:53.904275090+00:00 stderr P 2b. another ",
            "2019-05-07T18:57:54.904275091+00:00 stderr P multiline ",
            "2019-05-07T18:57:55.904275092+00:00 stderr F log",
            "2019-05-07T18:57:56.904275093+00:00 stdout F 3a. non multiline 1",
            "2019-05-07T18:57:57.904275094+00:00 stdout F 4a. non multiline 2",
            "2019-05-07T18:57:56.904275093+00:00 stderr F 3b. non multiline 1",
            "2019-05-07T18:57:57.904275094+00:00 stderr F 4b. non multiline 2",
        ];
        let want = [
            "2019-05-07T18:57:50.904275087+00:00 stdout P 1a. some multiline log",
            "2019-05-07T18:57:50.904275087+00:00 stderr P 1b. some multiline log",
            "2019-05-07T18:57:53.904275090+00:00 stdout P 2a. another multiline log",
            "2019-05-07T18:57:53.904275090+00:00 stderr P 2b. another multiline log",
            "2019-05-07T18:57:56.904275093+00:00 stdout F 3a. non multiline 1",
            "2019-05-07T18:57:57.904275094+00:00 stdout F 4a. non multiline 2",
            "2019-05-07T18:57:56.904275093+00:00 stderr F 3b. non multiline 1",
            "2019-05-07T18:57:57.904275094+00:00 stderr F 4b. non multiline 2",
        ];

        let reader = futures::stream::iter(input)
            .map(|line| Ok::<_, ()>((Bytes::from_static(line.as_bytes()), 1)));

        let multiline = Multiline::new(reader, Cri::default(), Duration::from_millis(200));

        let array = multiline.collect::<Vec<_>>().await;
        assert_eq!(array.len(), 8);

        for (item, want) in array.into_iter().zip(want) {
            let (got, _size) = item.unwrap();
            assert_eq!(got, want);
        }
    }
}
