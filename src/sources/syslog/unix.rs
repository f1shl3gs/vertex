use std::path::PathBuf;

use codecs::Decoder;
use codecs::decoding::{OctetCountingDecoder, SyslogDeserializer};
use configurable::Configurable;
use framework::Source;
use framework::config::{Resource, SourceContext};
use framework::source::unix::build_unix_stream_source;
use serde::{Deserialize, Serialize};
use value::OwnedValuePath;

use super::handle_events;

#[derive(Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Unix socket file path.
    path: PathBuf,
}

impl Config {
    pub fn resource(&self) -> Resource {
        Resource::UnixSocket(self.path.clone())
    }

    pub fn build(
        &self,
        cx: SourceContext,
        max_length: usize,
        host_key: OwnedValuePath,
    ) -> crate::Result<Source> {
        let decoder = Decoder::new(
            OctetCountingDecoder::new_with_max_length(max_length).into(),
            SyslogDeserializer::default().into(),
        );

        build_unix_stream_source(
            self.path.clone(),
            decoder,
            move |events, _received_from| handle_events(events, &host_key, None),
            cx.shutdown,
            cx.output,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_unix() {
        let config: Config = serde_yaml::from_str("path: /some/path/to/your.sock").unwrap();

        assert_eq!(config.path, PathBuf::from("/some/path/to/your.sock"));
    }
}
