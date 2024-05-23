use std::cmp::Ordering;
use std::net::SocketAddr;

use bytes::{Buf, BytesMut};
use event::LogRecord;
use framework::tls::MaybeTlsIncomingStream;
use framework::Pipeline;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use value::path;

// Specs definition from 2.2.1 MQTT Control Packet:
//
// http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html#_Toc398718021
const MQTT_CONNECT: u8 = 1;
const MQTT_CONNACK: u8 = 2;
const MQTT_PUBLISH: u8 = 3;
const MQTT_PUBACK: u8 = 4;
const MQTT_PUBREC: u8 = 5;
const MQTT_PUBREL: u8 = 6;
const MQTT_PUBCOMP: u8 = 7;
const MQTT_PINGREQ: u8 = 12;
const MQTT_PINGRESP: u8 = 13;
const MQTT_DISCONNECT: u8 = 14;

/* QOS Flag status */
const MQTT_QOS_LEV0: u8 = 0; /* no reply      */
const MQTT_QOS_LEV1: u8 = 1; /* PUBACK packet */
const MQTT_QOS_LEV2: u8 = 2; /* PUBREC packet */

// Protocol version
const MQTT_VERSION_311: u8 = 4;

const fn type_name(typ: u8) -> &'static str {
    match typ {
        MQTT_CONNECT => "CONNECT",
        MQTT_CONNACK => "CONNACK",
        MQTT_PUBLISH => "PUBLISH",
        MQTT_PUBACK => "PUBACK",
        MQTT_PUBREC => "PUBREC",
        MQTT_PUBREL => "PUBREL",
        MQTT_PUBCOMP => "PUBCOMP",
        MQTT_PINGREQ => "PINGREQ",
        MQTT_PINGRESP => "PINGRESP",
        MQTT_DISCONNECT => "DISCONNECT",
        _ => "unknown",
    }
}

pub async fn serve_connection(
    peer: SocketAddr,
    mut conn: MaybeTlsIncomingStream<TcpStream>,
    mut output: Pipeline,
) {
    let mut buf = BytesMut::new();

    'RECV: loop {
        if let Err(err) = conn.read_buf(&mut buf).await {
            error!(message = "read packet failed", ?err, ?peer,);
            return;
        }

        if buf.len() < 2 {
            continue;
        }

        loop {
            let ctrl_byte = buf[0];
            let mut remaining = 0usize;
            let mut shift = 0;
            for pos in 1..buf.len() {
                let byte = buf[pos] as usize;
                remaining += (byte & 0x7F) << shift;

                // stop when continue bit is 0
                if byte & 0x80 == 0 {
                    let want = 1 + pos + remaining;
                    if buf.len() < want {
                        continue 'RECV;
                    }

                    buf.advance(1 + pos);

                    break;
                }

                shift += 7;

                // Only a max of 4 bytes allowed for remaining length
                // more than 4 shifts(0, 7, 14, 21) implies bad length
                if shift > 21 {
                    error!(message = "invalid remaining length");
                    return;
                }
            }

            // handle packets
            let mut payload = buf.split_to(remaining).freeze();
            match ctrl_byte >> 4 {
                MQTT_CONNECT => {
                    //   PROTOCOL NAME
                    // byte     description
                    //   1        Protocol Name MSB
                    //   2        Protocol Name LSB
                    //   3        `M`
                    //   4        `Q`
                    //   5        `T`
                    //   6        `T`
                    //   7        Protocol version, 4 for MQTT311, 5 for MQTT5
                    //   8        Connect Flags
                    //   9        Keepalive MSB
                    //   10       Keepalive LSB
                    //   11
                    //   12
                    let mut len = payload[0] as usize;
                    len |= payload[1] as usize;

                    if len != 4 || payload[2..6].cmp(b"MQTT") != Ordering::Equal {
                        error!(message = "unknown protocol name");
                        return;
                    }

                    let version = payload[6];
                    if payload[6] != MQTT_VERSION_311 {
                        error!(message = "unsupported MQTT version", version);
                        return;
                    }

                    if let Err(err) = conn.write_all(&[MQTT_CONNACK << 4, 2, 0, 0]).await {
                        error!(message = "write CONNACK failed", ?err, ?peer);
                        return;
                    }
                }
                MQTT_PUBLISH => {
                    let mut len = payload[0] as usize;
                    len |= payload[1] as usize;
                    payload.advance(2);

                    let topic = match String::from_utf8(payload[..len].to_vec()) {
                        Ok(s) => {
                            payload.advance(len);
                            s
                        }
                        Err(err) => {
                            error!(message = "invalid topic name", ?err, ?peer);
                            return;
                        }
                    };

                    let qos = (ctrl_byte >> 1) & 0x03;
                    if qos > MQTT_QOS_LEV0 {
                        // packet identifier
                        //
                        // The Packet Identifier field is only present in
                        // `PUBLISH` Packets where the QoS level is 1 or 2.
                        //
                        // set the identifier that we are replying to
                        let mut resp = [0u8, 2, payload[0], payload[1]];

                        if qos == MQTT_QOS_LEV1 {
                            resp[0] = MQTT_PUBACK << 4;
                        } else if qos == MQTT_QOS_LEV2 {
                            resp[0] = MQTT_PUBREC << 4;
                        }

                        if let Err(err) = conn.write_all(&resp).await {
                            error!(message = "write PUBLISH response failed", ?err, ?peer);
                            return;
                        }

                        payload.advance(2);
                    }

                    let value: event::log::Value = serde_json::from_slice(&payload).unwrap();
                    let mut log = LogRecord::from(value);
                    log.metadata_mut()
                        .value_mut()
                        .insert(path!("topic"), topic.to_string());

                    if let Err(err) = output.send(log).await {
                        warn!(message = "send message failed", ?err, ?peer);
                        return;
                    }
                }
                MQTT_PINGREQ => {
                    let resp = [MQTT_PINGRESP >> 4, 0];
                    if let Err(err) = conn.write(&resp).await {
                        error!(message = "wrtie PINGRESP failed", ?err, ?peer);
                        return;
                    }
                }
                MQTT_DISCONNECT => {
                    debug!(message = "client disconnect", ?peer);
                    return;
                }
                typ => {
                    error!(
                        message = "unsupported packet type",
                        ?peer,
                        name = type_name(typ),
                        r#typ
                    );
                    return;
                }
            }

            if buf.is_empty() {
                // reuse buf
                buf.clear();
                break;
            }
        }
    }
}
