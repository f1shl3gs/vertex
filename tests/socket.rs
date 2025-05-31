mod util;

use std::str::FromStr;

use codecs::EncodingConfigWithFraming;
use codecs::encoding::{FramingConfig, JsonSerializerConfig, SerializerConfig};
use framework::sink::tcp::TcpSinkConfig;
use framework::testing::CountReceiver;
use testify::wait::wait_for_tcp;
use testify::{next_addr, send_lines};
use vertex::sinks::socket as socket_sink;
use vertex::sources::socket::Config as SocketConfig;

use crate::util::{start_topology, trace_init};

#[tokio::test]
async fn tcp_to_tcp() {
    trace_init();

    let num = 10000;

    let in_addr = next_addr();
    let out_addr = next_addr();

    let mut config = framework::config::Config::builder();
    config.add_source("in", SocketConfig::simple_tcp(in_addr));
    config.add_sink(
        "out",
        &["in"],
        socket_sink::Config::new(socket_sink::Mode::Tcp {
            config: TcpSinkConfig::from_address(out_addr.to_string()),
            encoding: EncodingConfigWithFraming::new(
                Some(FramingConfig::NewlineDelimited),
                SerializerConfig::Json(JsonSerializerConfig { pretty: false }),
                Default::default(),
            ),
        }),
    );

    let output_lines = CountReceiver::receive_lines(out_addr);

    let (topology, _crash) = start_topology(config.build().unwrap(), false).await;
    // Wait for server to accept traffic
    wait_for_tcp(in_addr).await;

    let input_messages: Vec<String> = (0..num)
        .map(|i| {
            // simple message is enough
            format!("{{\"seq\": {},\"key\": \"value\"}}", i)
        })
        .collect();

    let input_lines: Vec<String> = input_messages.iter().map(|msg| msg.to_string()).collect();

    send_lines(in_addr, input_lines).await.unwrap();

    // Shut down server
    topology.stop().await;

    let output_lines = output_lines.await;
    assert_eq!(output_lines.len(), num);

    let output_messages: Vec<_> = output_lines
        .iter()
        .map(|s| {
            let mut value = serde_json::Value::from_str(s).unwrap();

            let value = value.as_object_mut().unwrap().remove("message").unwrap();

            value.as_str().unwrap().to_string()
        })
        .collect();

    assert_eq!(input_messages, output_messages);
}
