use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Encodable;
use crate::channel::limited;
use crate::disk::LedgerError;
use crate::receiver::BufferReceiver;
use crate::sender::BufferSender;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Ledger(LedgerError),

    #[error("create directory {0:?} failed, {1}")]
    CreateRootDirectory(PathBuf, std::io::Error),

    #[error("build disk buffer, {0}")]
    Io(std::io::Error),
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

#[derive(Clone, Debug, PartialEq)]
pub enum BufferType {
    Memory {
        /// The maximum size of the buffer can hold. It works for Memory and Disk
        max_size: usize,
    },

    Disk {
        /// The maximum size of the buffer can hold. It works for Memory and Disk
        max_size: usize,

        /// The size limitation of each Record
        max_record_size: usize,

        /// The size limitation of chunk file
        max_chunk_size: usize,
    },
}

#[derive(Debug, Default, PartialEq)]
pub struct BufferConfig {
    pub when_full: WhenFull,

    pub typ: BufferType,
}

// serde's `default` and `flatten` attribute cannot work together, so here we are, see
//
// https://github.com/serde-rs/serde/issues/1626
// https://github.com/serde-rs/serde/pull/2687
// https://github.com/serde-rs/serde/pull/2751
mod _serde {
    use std::fmt::Formatter;

    use serde::de::{Error, MapAccess, Unexpected, Visitor};
    use serde::ser::SerializeStruct;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::{BufferConfig, BufferType, WhenFull};

    const DEFAULT_MEMORY_MAX_SIZE: usize = 8 * 1024 * 1024; // 8MB
    const DEFAULT_MAX_RECORD_SIZE: usize = 8 * 1024 * 1024; // 8MB
    const DEFAULT_MAX_CHUNK_FILE_SIZE: usize = 128 * 1024 * 1024; // 128MB
    const DEFAULT_DISK_BUFFER_SIZE: usize = 64 * 1024 * 1024 * 1024; // 64GB

    fn default_memory_max_size() -> usize {
        DEFAULT_MEMORY_MAX_SIZE
    }

    fn default_disk_max_size() -> usize {
        DEFAULT_DISK_BUFFER_SIZE
    }

    fn default_max_chunk_size() -> usize {
        DEFAULT_MAX_CHUNK_FILE_SIZE
    }

    fn default_max_record_size() -> usize {
        DEFAULT_MAX_RECORD_SIZE
    }

    impl Default for BufferType {
        fn default() -> Self {
            BufferType::Memory {
                max_size: DEFAULT_MEMORY_MAX_SIZE,
            }
        }
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct Memory {
        #[serde(default = "default_memory_max_size", with = "humanize::bytes::serde")]
        max_size: usize,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct Disk {
        #[serde(default = "default_disk_max_size", with = "humanize::bytes::serde")]
        max_size: usize,

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
                    let mut typ = None;
                    let mut when_full = WhenFull::default();

                    while let Some(key) = map.next_key::<&str>()? {
                        match key {
                            "when_full" => {
                                when_full = map.next_value::<WhenFull>()?;
                            }
                            "memory" => match typ {
                                None => {
                                    let memory = map.next_value::<Memory>()?;

                                    typ = Some(BufferType::Memory {
                                        max_size: memory.max_size,
                                    });
                                }
                                Some(BufferType::Memory { .. }) => {
                                    return Err(Error::duplicate_field("memory"));
                                }
                                Some(BufferType::Disk { .. }) => {
                                    return Err(Error::custom("disk buffer is already defined"));
                                }
                            },
                            "disk" => match typ {
                                None => {
                                    let disk = map.next_value::<Disk>()?;

                                    typ = Some(BufferType::Disk {
                                        max_chunk_size: disk.max_chunk_size,
                                        max_size: disk.max_size,
                                        max_record_size: disk.max_record_size,
                                    });
                                }
                                Some(BufferType::Memory { .. }) => {
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
                        when_full,
                        typ: typ.unwrap_or(BufferType::Memory {
                            max_size: DEFAULT_MEMORY_MAX_SIZE,
                        }),
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
            let mut s = serializer.serialize_struct("BufferConfig", 2)?;

            s.serialize_field("when_full", &self.when_full)?;

            // skip serialize if default
            if self.typ == BufferType::default() {
                return s.end();
            }

            match self.typ {
                BufferType::Memory { max_size } => {
                    s.serialize_field("memory", &Memory { max_size })?;
                }
                BufferType::Disk {
                    max_size,
                    max_chunk_size,
                    max_record_size,
                } => {
                    s.serialize_field(
                        "disk",
                        &Disk {
                            max_size,
                            max_chunk_size,
                            max_record_size,
                        },
                    )?;
                }
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
        id: String,
        root: PathBuf,
    ) -> Result<(BufferSender<T>, BufferReceiver<T>), Error> {
        match self.typ {
            BufferType::Memory { max_size } => {
                let (tx, rx) = limited(max_size);

                Ok((
                    BufferSender::memory(tx, self.when_full),
                    BufferReceiver::Memory(rx),
                ))
            }
            BufferType::Disk {
                max_size,
                max_chunk_size,
                max_record_size,
            } => {
                let config = crate::disk::Config {
                    root: root.join(id),
                    max_record_size,
                    max_chunk_size,
                    max_buffer_size: max_size,
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
memory:
    max_size: 16 mi
";

        let config = serde_yaml::from_str::<BufferConfig>(simple).unwrap();
        assert_eq!(config.when_full, WhenFull::default());
        assert!(
            matches!(config.typ, BufferType::Memory { max_size } if max_size == 16 * 1024 * 1024 )
        );

        let simple_with_mode = r"
when_full: drop_newest
memory:
    max_size: 16 mi
        ";

        let config = serde_yaml::from_str::<BufferConfig>(simple_with_mode).unwrap();
        assert_eq!(config.when_full, WhenFull::DropNewest);
        assert!(
            matches!(config.typ, BufferType::Memory { max_size } if max_size == 16 * 1024 * 1024 )
        );

        let custom_disk = r"
        disk:
            max_size: 16 gib
            max_chunk_size: 256 mi
        ";
        let config = serde_yaml::from_str::<BufferConfig>(custom_disk).unwrap();
        assert_eq!(config.when_full, WhenFull::default());
        assert!(matches!(
            config.typ,
            BufferType::Disk { max_chunk_size, max_size, .. } if max_chunk_size == 256 * 1024 * 1024 && max_size == 16 * 1024 * 1024 * 1024,
        ));
    }
}
