use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Encodable;
use crate::channel::limited;
use crate::disk::LedgerError;
use crate::receiver::BufferReceiver;
use crate::sender::BufferSender;

const DEFAULT_MAX_SIZE: usize = 8 * 1024 * 1024; // 8MB

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Ledger(LedgerError),

    #[error("create directory {0:?} failed, {1}")]
    CreateRootDirectory(PathBuf, std::io::Error),

    #[error("open reader failed, {0}")]
    Reader(std::io::Error),

    #[error("open writer failed, {0}")]
    Writer(std::io::Error),
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WhenFull {
    /// Wait for free space in the buffer.
    ///
    /// This applies backpressure up the topology, signalling that sources should slow down
    /// the acceptance/consumption of events. This means that while no data is lost, data will pile
    /// up at the edge.
    #[default]
    Block,

    /// Drops the event instead of waiting for free space in buffer.
    ///
    /// The event will be intentionally dropped. This mode is typically used when performance is the
    /// highest priority, and it is preferable to temporarily lose events rather than cause a
    /// slowdown in the acceptance/consumption of events.
    DropNewest,
    // /// Drops the oldest event, instead of waiting for free space in buffer.
    // DropOldest,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum BufferType {
    #[default]
    Memory,

    Disk {
        /// The size limitation of each Record
        max_record_size: usize,

        /// The size limitation of chunk file
        max_chunk_size: usize,
    },
}

#[derive(Debug, PartialEq)]
pub struct BufferConfig {
    /// The maximum size of the buffer can hold. It works for Memory and Disk
    pub max_size: usize,

    pub when_full: WhenFull,

    pub typ: BufferType,
}

impl Default for BufferConfig {
    fn default() -> Self {
        BufferConfig {
            max_size: DEFAULT_MAX_SIZE,
            when_full: WhenFull::default(),
            typ: BufferType::Memory,
        }
    }
}

// serde's `default` and `flatten` attribute cannot work together, so here we are, see
//
// https://github.com/serde-rs/serde/issues/1626
// https://github.com/serde-rs/serde/pull/2687
// https://github.com/serde-rs/serde/pull/2751
mod _serde {
    use std::borrow::Cow;
    use std::fmt::Formatter;

    use serde::de::{Error, MapAccess, Unexpected, Visitor};
    use serde::ser::SerializeStruct;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::{BufferConfig, BufferType, DEFAULT_MAX_SIZE, WhenFull};

    const DEFAULT_MAX_RECORD_SIZE: usize = 8 * 1024 * 1024; // 8MB
    const DEFAULT_MAX_CHUNK_FILE_SIZE: usize = 128 * 1024 * 1024; // 128MB

    fn default_max_chunk_size() -> usize {
        DEFAULT_MAX_CHUNK_FILE_SIZE
    }

    fn default_max_record_size() -> usize {
        DEFAULT_MAX_RECORD_SIZE
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct Disk {
        /// The max size of each chunk, events will be written into chunks until
        /// the size of chunks become this size
        #[serde(default = "default_max_chunk_size", with = "humanize::bytes::serde")]
        max_chunk_size: usize,

        #[serde(default = "default_max_record_size", with = "humanize::bytes::serde")]
        max_record_size: usize,
    }

    impl<'de> Deserialize<'de> for BufferConfig {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct ConfigVisitor;

            impl<'de> Visitor<'de> for ConfigVisitor {
                type Value = BufferConfig;

                fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                    formatter.write_str("a buffer config")
                }

                fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    let mut max_size = DEFAULT_MAX_SIZE;
                    let mut when_full = WhenFull::default();
                    let mut typ = None;

                    while let Some(key) = map.next_key::<&str>()? {
                        match key {
                            "max_size" => {
                                let value = map.next_value::<Cow<str>>()?;
                                max_size = humanize::bytes::parse_bytes(value.as_ref())
                                    .map_err(A::Error::custom)?;
                            }
                            "when_full" => {
                                when_full = map.next_value::<WhenFull>()?;
                            }
                            "disk" => match typ {
                                None => {
                                    let disk = map.next_value::<Disk>()?;

                                    typ = Some(BufferType::Disk {
                                        max_chunk_size: disk.max_chunk_size,
                                        max_record_size: disk.max_record_size,
                                    });
                                }
                                Some(BufferType::Memory) => {
                                    return Err(Error::custom("memory buffer is already defined"));
                                }
                                Some(BufferType::Disk { .. }) => {
                                    return Err(Error::duplicate_field("disk"));
                                }
                            },
                            key => {
                                return Err(Error::unknown_field(
                                    key,
                                    &["when_full", "memory", "disk"],
                                ));
                            }
                        }
                    }

                    if let Some(BufferType::Disk { max_chunk_size, .. }) = &typ
                        && *max_chunk_size > 1024 * 1024 * 1024
                    {
                        return Err(Error::invalid_value(
                            Unexpected::Unsigned(*max_chunk_size as u64),
                            &"an unsigned value less than 1G",
                        ));
                    }

                    Ok(BufferConfig {
                        max_size,
                        when_full,
                        typ: typ.unwrap_or(BufferType::Memory),
                    })
                }
            }

            deserializer.deserialize_map(ConfigVisitor)
        }
    }

    impl Serialize for BufferConfig {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut s = serializer.serialize_struct("BufferConfig", 3)?;

            // skip serialize if default
            if self.max_size != DEFAULT_MAX_SIZE {
                s.serialize_field("max_size", &self.max_size)?;
            }

            if self.when_full != WhenFull::default() {
                s.serialize_field("when_full", &self.when_full)?;
            }

            if let BufferType::Disk {
                max_chunk_size,
                max_record_size,
            } = &self.typ
            {
                s.serialize_field(
                    "disk",
                    &Disk {
                        max_chunk_size: *max_chunk_size,
                        max_record_size: *max_record_size,
                    },
                )?;
            }

            s.end()
        }
    }
}

impl BufferConfig {
    /// Builds the buffer components represented by this configuration.
    ///
    /// The caller gets back a `Sink` and `Stream` implementation that represent a way
    /// to push items into the buffer, as well as pop items out of the buffer, respectively.
    /// The `Acker` is provided to callers in order to update the buffer when popped items
    /// have been processed and can be dropped or deleted, depending on the underlying
    /// buffer implementation.
    pub fn build<T: Encodable + Unpin>(
        &self,
        id: &str,
        root: PathBuf,
    ) -> Result<(BufferSender<T>, BufferReceiver<T>), Error> {
        match self.typ {
            BufferType::Memory => {
                let (tx, rx) = limited(self.max_size);

                Ok((
                    BufferSender::memory(tx, self.when_full),
                    BufferReceiver::Memory(rx),
                ))
            }
            BufferType::Disk {
                max_chunk_size,
                max_record_size,
            } => {
                let config = crate::disk::Config {
                    root: root.join(id),
                    max_record_size,
                    max_chunk_size,
                    max_buffer_size: self.max_size,
                };

                let (writer, reader) = config.build()?;

                Ok((
                    BufferSender::disk(writer, self.when_full),
                    BufferReceiver::Disk(reader),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let simple = r"
max_size: 16 mi
";

        let config = serde_yaml::from_str::<BufferConfig>(simple).unwrap();
        assert_eq!(config.max_size, 16 * 1024 * 1024);
        assert_eq!(config.when_full, WhenFull::default());
        assert!(matches!(config.typ, BufferType::Memory));

        let simple_with_mode = r"
max_size: 16 mi
when_full: drop_newest
        ";

        let config = serde_yaml::from_str::<BufferConfig>(simple_with_mode).unwrap();
        assert_eq!(config.max_size, 16 * 1024 * 1024);
        assert_eq!(config.when_full, WhenFull::DropNewest);
        assert!(matches!(config.typ, BufferType::Memory if config.max_size == 16 * 1024 * 1024));

        let custom_disk = r"
        max_size: 16 gib
        disk:
            max_chunk_size: 256 mi
        ";
        let config = serde_yaml::from_str::<BufferConfig>(custom_disk).unwrap();
        assert_eq!(config.max_size, 16 * 1024 * 1024 * 1024);
        assert_eq!(config.when_full, WhenFull::default());
        assert!(matches!(
            config.typ,
            BufferType::Disk { max_chunk_size, .. } if max_chunk_size == 256 * 1024 * 1024,
        ));
    }
}
