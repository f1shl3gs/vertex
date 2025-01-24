#![allow(warnings)]

use std::net::SocketAddr;
use std::time::Duration;

use configurable::generate_config_with_schema;
use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use serde::Deserialize;

#[test]
fn flatten_all() {
    #[derive(Configurable, Deserialize)]
    struct Config {
        #[serde(flatten)]
        mode: Mode,
        ack: bool,
    }

    #[derive(Configurable, Deserialize)]
    struct TcpConfig {
        address: SocketAddr,
        keepalive: Option<Duration>,
    }

    #[derive(Configurable, Deserialize)]
    struct EncodingConfig {
        codec: String,
        pretty: bool,
        frame: String,
    }

    #[derive(Configurable, Deserialize)]
    #[serde(tag = "mode", rename_all = "snake_case")]
    enum Mode {
        Tcp {
            /// Tcp Config
            #[serde(flatten)]
            config: TcpConfig,

            encoding: EncodingConfig,
        },
    }

    let root_schema = generate_root_schema::<Config>();
    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let example = generate_config_with_schema(root_schema);
    println!("{}", example);
}
