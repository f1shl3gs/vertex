use async_trait::async_trait;
use event::trace::Trace;
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
    let process = jaeger::Process::new(trace.service.to_string(), Some(tags));
    let spans = trace.spans.into_iter().map(Into::into).collect();

    jaeger::Batch::new(process, spans)
}
