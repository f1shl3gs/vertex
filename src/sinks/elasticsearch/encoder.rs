use std::io;
use std::io::Write;

use bytesize::ByteSizeOf;
use codecs::encoding::Transformer;
use event::{Event, EventFinalizers, Finalizable, LogRecord};
use framework::sink::util::{Encoder, as_tracked_write};

use super::BulkAction;

#[derive(Debug, Clone, PartialEq)]
pub struct ElasticsearchEncoder {
    pub doc_type: String,
    pub suppress_type_name: bool,
    pub transformer: Transformer,
}

pub struct ProcessedEvent {
    pub index: String,
    pub bulk_action: BulkAction,
    pub log: LogRecord,
    pub id: Option<String>,
}

impl Finalizable for ProcessedEvent {
    fn take_finalizers(&mut self) -> EventFinalizers {
        self.log.metadata_mut().take_finalizers()
    }
}

impl ByteSizeOf for ProcessedEvent {
    fn allocated_bytes(&self) -> usize {
        self.index.allocated_bytes() + self.log.allocated_bytes() + self.id.allocated_bytes()
    }
}

impl Encoder<Vec<ProcessedEvent>> for ElasticsearchEncoder {
    fn encode(&self, input: Vec<ProcessedEvent>, writer: &mut dyn Write) -> io::Result<usize> {
        let mut written = 0;

        for event in input {
            let log = {
                let mut event = Event::from(event.log);
                self.transformer.transform(&mut event);
                event.into_log()
            };

            written += write_bulk_action(
                writer,
                event.bulk_action.as_str(),
                &event.index,
                &self.doc_type,
                self.suppress_type_name,
                &event.id,
            )?;
            written +=
                as_tracked_write::<_, _, io::Error>(writer, log.value(), |mut writer, log| {
                    writer.write_all(b"\n")?;
                    #[allow(clippy::needless_borrows_for_generic_args)]
                    serde_json::to_writer(&mut writer, log)?;
                    writer.write_all(b"\n")?;
                    Ok(())
                })?;
        }

        Ok(written)
    }
}

fn write_bulk_action(
    writer: &mut dyn Write,
    bulk_action: &str,
    index: &str,
    doc_type: &str,
    suppress_type: bool,
    id: &Option<String>,
) -> io::Result<usize> {
    as_tracked_write(
        writer,
        (bulk_action, index, doc_type, id, suppress_type),
        |writer, (bulk_action, index, doc_type, id, suppress_type)| match (id, suppress_type) {
            (Some(id), true) => {
                write!(
                    writer,
                    r#"{{"{bulk_action}":{{"_index":"{index}","_id":"{id}"}}}}"#,
                )
            }
            (Some(id), false) => {
                write!(
                    writer,
                    r#"{{"{bulk_action}":{{"_index":"{index}","_type":"{doc_type}","_id":"{id}"}}}}"#,
                )
            }
            (None, true) => {
                write!(writer, r#"{{"{bulk_action}":{{"_index":"{index}"}}}}"#)
            }
            (None, false) => {
                write!(
                    writer,
                    r#"{{"{bulk_action}":{{"_index":"{index}","_type":"{doc_type}"}}}}"#,
                )
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suppress_type_with_id() {
        let mut writer = Vec::new();
        let _ = write_bulk_action(
            &mut writer,
            "ACTION",
            "INDEX",
            "TYPE",
            true,
            &Some("ID".to_string()),
        );

        let value: serde_json::Value = serde_json::from_slice(&writer).unwrap();
        let value = value.as_object().unwrap();

        assert!(value.contains_key("ACTION"));

        let nested = value.get("ACTION").unwrap();
        let nested = nested.as_object().unwrap();

        assert_eq!(nested.get("_index").unwrap().as_str(), Some("INDEX"));
        assert_eq!(nested.get("_id").unwrap().as_str(), Some("ID"));
        assert!(!nested.contains_key("_type"));
    }

    #[test]
    fn suppress_type_without_id() {
        let mut writer = Vec::new();

        let _ = write_bulk_action(&mut writer, "ACTION", "INDEX", "TYPE", true, &None);
        let value: serde_json::Value = serde_json::from_slice(&writer).unwrap();
        let value = value.as_object().unwrap();

        assert!(value.contains_key("ACTION"));

        let nested = value.get("ACTION").unwrap();
        let nested = nested.as_object().unwrap();

        assert!(nested.contains_key("_index"));
        assert_eq!(nested.get("_index").unwrap().as_str(), Some("INDEX"));
        assert!(!nested.contains_key("_id"));
        assert!(!nested.contains_key("_type"));
    }

    #[test]
    fn type_with_id() {
        let mut writer = Vec::new();
        let _ = write_bulk_action(
            &mut writer,
            "ACTION",
            "INDEX",
            "TYPE",
            false,
            &Some("ID".to_string()),
        );

        let value: serde_json::Value = serde_json::from_slice(&writer).unwrap();
        let value = value.as_object().unwrap();

        assert!(value.contains_key("ACTION"));

        let nested = value.get("ACTION").unwrap();
        let nested = nested.as_object().unwrap();

        assert_eq!(nested.get("_index").unwrap().as_str(), Some("INDEX"));
        assert_eq!(nested.get("_id").unwrap().as_str(), Some("ID"));
        assert_eq!(nested.get("_type").unwrap().as_str(), Some("TYPE"));
    }

    #[test]
    fn type_without_id() {
        let mut writer = Vec::new();
        let _ = write_bulk_action(&mut writer, "ACTION", "INDEX", "TYPE", false, &None);

        let value: serde_json::Value = serde_json::from_slice(&writer).unwrap();
        let value = value.as_object().unwrap();

        assert!(value.contains_key("ACTION"));

        let nested = value.get("ACTION").unwrap();
        let nested = nested.as_object().unwrap();

        assert_eq!(nested.get("_index").unwrap().as_str(), Some("INDEX"));
        assert!(!nested.contains_key("_id"));
        assert_eq!(nested.get("_type").unwrap().as_str(), Some("TYPE"));
    }
}
