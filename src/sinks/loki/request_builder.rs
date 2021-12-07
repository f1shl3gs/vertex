use std::collections::HashMap;
use std::io::Write;
use serde::{Serialize, Serializer, ser::SerializeSeq};
use event::{ByteSizeOf, EventFinalizers, Finalizable};
use event::encoding::Encoder;

pub type Labels = Vec<(String, String)>;

#[derive(Clone, Debug)]
pub struct LokiRecord {
    pub partition: PartitionKey,
    pub labels: Labels,
    pub event: LokiEvent,
    pub finalizers: EventFinalizers,
}

impl ByteSizeOf for LokiRecord {
    fn allocated_bytes(&self) -> usize {
        self.partition.allocated_bytes()
            + self.labels.iter().fold(0, |acc, (k, v)| {
            acc + k.allocated_bytes() + v.allocated_bytes()
        })
            + self.event.allocated_bytes()
    }
}

impl Finalizable for LokiRecord {
    fn take_finalizers(&mut self) -> EventFinalizers {
        std::mem::take(&mut self.finalizers)
    }
}


#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct PartitionKey {
    pub tenant: Option<String>,
    labels: String,
}

impl ByteSizeOf for PartitionKey {
    fn allocated_bytes(&self) -> usize {
        self.tenant
            .as_ref()
            .map(|value| value.allocated_bytes())
            .unwrap_or(0)
            + self.labels.allocated_bytes()
    }
}

impl PartitionKey {
    pub fn new(tenant: Option<String>, labels: &mut Labels) -> Self {
        // Let's join all of the labels to single string so that cloning requires only one
        // single allocation. that requires sorting to ensure uniqueness, but also choosing
        // a separator that isn't likely to be used in either name or value.
        labels.sort();
        PartitionKey {
            tenant,
            labels: labels.iter()
                .flat_map(|(k, v)| [k, "→", v, "∇"]).collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LokiEvent {
    pub timestamp: i64,
    pub event: String,
}

impl ByteSizeOf for LokiEvent {
    fn allocated_bytes(&self) -> usize {
        self.timestamp.allocated_bytes() + self.event.allocated_bytes()
    }
}

impl Serialize for LokiEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&self.timestamp.to_string())?;
        seq.serialize_element(&self.event)?;
        seq.end()
    }
}

// This struct is PushRequest
// https://github.com/grafana/loki/blob/main/pkg/logproto/logproto.proto#L29
#[derive(Debug, Default, Serialize)]
pub struct LokiBatch {
    stream: HashMap<String, String>,
    values: Vec<LokiEvent>,

    #[serde(skip)]
    finalizers: EventFinalizers,
}

impl From<Vec<LokiRecord>> for LokiBatch {
    fn from(records: Vec<LokiRecord>) -> Self {
        let mut batch = records.into_iter()
            .fold(Self::default(), |mut batch, mut record| {
                batch.finalizers.merge(record.take_finalizers());
                batch.stream.extend(record.labels.into_iter());
                batch.values.push(record.event);
                batch
            });

        batch.values.sort_by_key(|event| event.timestamp);
        batch
    }
}

#[derive(Clone, Default)]
pub struct LokiBatchEncoder;

impl Encoder<Vec<LokiRecord>> for LokiBatchEncoder {
    fn encode(&self, input: Vec<LokiRecord>, writer: &mut dyn Write) -> std::io::Result<usize> {
        let batch = LokiBatch::from(input);
        let body = serde_json::json!({"stream": [batch]});
        let body = serde_json::to_vec(&body)?;
        writer.write(&body)
    }
}

