use async_trait::async_trait;
use event::trace::{
    EvictedHashMap, EvictedQueue, Key, KeyValue, Link, SpanKind, StatusCode, Trace,
};
use framework::config::{DataType, GenerateConfig, SinkConfig, SinkContext, SinkDescription};
use framework::sink::util::udp::UdpSinkConfig;
use framework::{Healthcheck, Sink};
use jaeger::agent::{serialize_batch, BufferClient};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CollectorConfig {}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "propagator", rename_all = "snake_case")]
enum Mode {
    Agent(UdpSinkConfig),
    Collector(CollectorConfig),
}

#[derive(Debug, Deserialize, Serialize)]
struct JaegerConfig {
    #[serde(flatten)]
    pub mode: Mode,
}

impl GenerateConfig for JaegerConfig {
    fn generate_config() -> String {
        r#"
# The type jaeger compoent
#
propagator: agent

# The address to connect to. The address must include a port.
address: localhost:6831

"#
        .into()
    }
}

inventory::submit! {
    SinkDescription::new::<JaegerConfig>("jaeger")
}

#[async_trait]
#[typetag::serde(name = "jaeger")]
impl SinkConfig for JaegerConfig {
    async fn build(&self, cx: SinkContext) -> framework::Result<(Sink, Healthcheck)> {
        match &self.mode {
            Mode::Agent(config) => {
                config.build(cx, move |event| {
                    // TODO: This buffer_client is dummy, rework it in the future
                    let mut buffer_client = BufferClient::default();

                    let trace = event.into_trace();
                    let batch = trace_to_batch(trace);

                    match serialize_batch(
                        &mut buffer_client,
                        batch,
                        jaeger::agent::UDP_PACKET_MAX_LENGTH,
                    ) {
                        Ok(data) => Some(data.into()),
                        Err(err) => {
                            warn!(message = "Encode batch failed", ?err);

                            None
                        }
                    }
                })
            }
            _ => unimplemented!(),
        }
    }

    fn input_type(&self) -> DataType {
        DataType::Trace
    }

    fn sink_type(&self) -> &'static str {
        "jaeger"
    }
}

fn trace_to_batch(trace: Trace) -> jaeger::Batch {
    let tags = trace
        .tags
        .into_iter()
        .map(|(k, v)| {
            jaeger::Tag::new(
                k.into(),
                jaeger::TagType::String,
                Some(v.into()),
                None,
                None,
                None,
                None,
            )
        })
        .collect();
    let process = jaeger::Process::new(trace.service, Some(tags));
    let spans = trace
        .spans
        .into_iter()
        .map(|span| {
            let trace_id_bytes = span.trace_id().unwrap().to_bytes();
            let (high, low) = trace_id_bytes.split_at(8);
            let trace_id_high = i64::from_be_bytes(high.try_into().unwrap());
            let trace_id_low = i64::from_be_bytes(low.try_into().unwrap());

            jaeger::Span {
                trace_id_low,
                trace_id_high,
                span_id: span.span_id().into_i64(),
                parent_span_id: span.parent_span_id.into_i64(),
                operation_name: "".to_string(),
                references: links_to_references(span.links),
                flags: 0,
                start_time: span.start_time,
                duration: span.end_time - span.start_time,
                tags: Some(build_span_tags(
                    span.attributes,
                    span.status.status_code,
                    span.status.message.into_owned(),
                    span.kind,
                )),
                logs: None,
            }
        })
        .collect();

    jaeger::Batch::new(process, spans)
}

const ERROR: &str = "error";
const SPAN_KIND: &str = "span.kind";
const OTEL_STATUS_CODE: &str = "otel.status_code";
const OTEL_STATUS_DESCRIPTION: &str = "otel.status_description";

#[derive(Default)]
struct UserOverrides {
    error: bool,
    span_kind: bool,
    status_code: bool,
    status_description: bool,
}

impl UserOverrides {
    fn record_attr(&mut self, attr: &str) {
        match attr {
            ERROR => self.error = true,
            SPAN_KIND => self.span_kind = true,
            OTEL_STATUS_CODE => self.status_code = true,
            OTEL_STATUS_DESCRIPTION => self.status_description = true,
            _ => (),
        }
    }
}

fn build_span_tags(
    attrs: EvictedHashMap,
    status_code: StatusCode,
    status_description: String,
    kind: SpanKind,
) -> Vec<jaeger::Tag> {
    let mut user_overrides = UserOverrides::default();
    // TODO: determine if namespacing is required to avoid collision with set attributes
    let mut tags = attrs
        .into_iter()
        .map(|(k, v)| {
            user_overrides.record_attr(k.as_str());
            KeyValue::new(k, v).into()
        })
        .collect::<Vec<_>>();

    if !user_overrides.span_kind && kind != SpanKind::Internal {
        tags.push(Key::new(SPAN_KIND).string(kind.to_string()).into())
    }

    if status_code != StatusCode::Unset {
        // Ensure error status is set unless user has already overrided it
        if status_code == StatusCode::Error {
            tags.push(Key::new(ERROR).bool(true).into());
        }

        if !user_overrides.status_code {
            tags.push(
                Key::new(OTEL_STATUS_CODE)
                    .string::<&'static str>(status_code.as_str())
                    .into(),
            );
        }

        // set status message if there is one
        if !status_description.is_empty() && !user_overrides.status_description {
            tags.push(
                Key::new(OTEL_STATUS_DESCRIPTION)
                    .string(status_description)
                    .into(),
            );
        }
    }

    tags
}

fn links_to_references(links: EvictedQueue<Link>) -> Option<Vec<jaeger::SpanRef>> {
    if links.is_empty() {
        return None;
    }

    let refs = links
        .iter()
        .map(|link| {
            let trace_id_bytes = link.trace_id.to_bytes();
            let (high, low) = trace_id_bytes.split_at(8);
            let trace_id_high = i64::from_be_bytes(high.try_into().unwrap());
            let trace_id_low = i64::from_be_bytes(low.try_into().unwrap());

            jaeger::SpanRef::new(
                jaeger::SpanRefType::FollowsFrom,
                trace_id_low,
                trace_id_high,
                link.span_id.into_i64(),
            )
        })
        .collect();

    Some(refs)
}
