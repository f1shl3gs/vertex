use std::path::PathBuf;

use bytes::Bytes;
use codecs::Decoder;
use codecs::decoding::{DeserializerConfig, FramingConfig};
use configurable::Configurable;
use event::Events;
use framework::Source;
use framework::config::{Resource, SourceContext};
use framework::source::unix::{build_unix_datagram_source, build_unix_stream_source};
use serde::{Deserialize, Serialize};

use crate::sources::default_decoding;

/// Unix domain socket configuration for the `socket` source.
#[derive(Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    /// The Unix socket path
    ///
    /// This field should be an absolute path.
    pub path: PathBuf,

    /// Unix file mode bits to be applied to the unix socket file as its
    /// designated file permissions.
    ///
    /// Note: The file mode value can be specified in any numeric format
    /// supported by your configuration language, but it is most intuitive
    /// to use an octal number
    #[configurable(example = "0o777")]
    pub permissions: Option<u32>,

    #[serde(default)]
    pub framing: Option<FramingConfig>,

    #[serde(default = "default_decoding")]
    pub decoding: DeserializerConfig,
}

impl Config {
    pub fn resource(&self) -> Resource {
        Resource::UnixSocket(self.path.to_string_lossy().to_string())
    }

    pub fn run_stream(&self, decoder: Decoder, cx: SourceContext) -> crate::Result<Source> {
        build_unix_stream_source(
            self.path.clone(),
            decoder,
            move |events, _size| handle_events(events, None),
            cx.shutdown,
            cx.output,
        )
    }

    pub fn run_datagram(&self, decoder: Decoder, cx: SourceContext) -> crate::Result<Source> {
        let max_length = self.framing.as_ref().and_then(|framing| match framing {
            FramingConfig::NewlineDelimited(config) => config.max_length,
            FramingConfig::CharacterDelimited(config) => config.max_length,
            FramingConfig::OctetCounting(config) => config.max_length,
            _ => None,
        });

        build_unix_datagram_source(
            self.path.clone(),
            self.permissions,
            max_length.unwrap_or(128 * 1024),
            decoder,
            handle_events,
            cx.shutdown,
            cx.output,
        )
    }
}

fn handle_events(events: &mut Events, received_from: Option<&Bytes>) {
    // let now = Utc::now();

    events.for_each_log(|log| {
        log.insert("host", received_from.cloned());
    })
}
