mod netlink;
mod parse;
mod syscalls;

use std::time::Duration;

use configurable::configurable_component;
use event::LogRecord;
use framework::config::{Output, SourceConfig, SourceContext};
use framework::{Pipeline, ShutdownSignal, Source};
use futures::StreamExt;
use tokio_util::codec::LengthDelimitedCodec;

#[configurable_component(source, name = "audit")]
struct Config {}

#[async_trait::async_trait]
#[typetag::serde(name = "audit")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        // try connect, and return error if any
        let _conn = netlink::Connection::connect()?;

        Ok(Box::pin(run(cx.shutdown, cx.output)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

/// `run` start a read only connection, and receive audit events
async fn run(mut shutdown: ShutdownSignal, mut output: Pipeline) -> Result<(), ()> {
    loop {
        // connect first
        let conn = loop {
            match netlink::Connection::connect() {
                Ok(conn) => break conn,
                Err(err) => {
                    warn!(
                        message = "failed to establish netlink audit connection",
                        ?err
                    );
                }
            }

            // backoff
            tokio::select! {
                _ = &mut shutdown => return Ok(()),
                _ = tokio::time::sleep(Duration::from_secs(5)) => {}
            }
        };

        let mut reader = LengthDelimitedCodec::builder()
            .native_endian()
            .length_field_type::<u32>()
            .num_skip(0)
            .new_read(conn);

        loop {
            let buf = tokio::select! {
                _ = &mut shutdown => return Ok(()),
                result = reader.next() => match result {
                    Some(Ok(buf)) => buf,
                    Some(Err(err)) => {
                        warn!(
                            message = "listen for audit log failed",
                            ?err
                        );

                        break
                    },
                    None => break,
                }
            };

            let value = match parse::parse(&buf) {
                Ok(value) => value,
                Err(err) => {
                    warn!(message = "parse audit log failed", ?err);

                    continue;
                }
            };

            if let Err(err) = output.send(LogRecord::from(value)).await {
                warn!(message = "send audit log failed", ?err);

                return Err(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn length_delimit() {
        let reader = Cursor::new([
            243, 0, 0, 0, 107, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 97, 117, 100, 105, 116, 40, 49, 55,
            52, 52, 57, 56, 55, 51, 48, 50, 46, 57, 53, 57, 58, 57, 54, 48, 48, 41, 58, 32, 112,
            105, 100, 61, 49, 32, 117, 105, 100, 61, 48, 32, 97, 117, 105, 100, 61, 52, 50, 57, 52,
            57, 54, 55, 50, 57, 53, 32, 115, 101, 115, 61, 52, 50, 57, 52, 57, 54, 55, 50, 57, 53,
            32, 115, 117, 98, 106, 61, 115, 121, 115, 116, 101, 109, 95, 117, 58, 115, 121, 115,
            116, 101, 109, 95, 114, 58, 105, 110, 105, 116, 95, 116, 58, 115, 48, 32, 109, 115,
            103, 61, 39, 117, 110, 105, 116, 61, 78, 101, 116, 119, 111, 114, 107, 77, 97, 110, 97,
            103, 101, 114, 45, 100, 105, 115, 112, 97, 116, 99, 104, 101, 114, 32, 99, 111, 109,
            109, 61, 34, 115, 121, 115, 116, 101, 109, 100, 34, 32, 101, 120, 101, 61, 34, 47, 117,
            115, 114, 47, 108, 105, 98, 47, 115, 121, 115, 116, 101, 109, 100, 47, 115, 121, 115,
            116, 101, 109, 100, 34, 32, 104, 111, 115, 116, 110, 97, 109, 101, 61, 63, 32, 97, 100,
            100, 114, 61, 63, 32, 116, 101, 114, 109, 105, 110, 97, 108, 61, 63, 32, 114, 101, 115,
            61, 115, 117, 99, 99, 101, 115, 115, 39, // duplicate
            243, 0, 0, 0, 107, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 97, 117, 100, 105, 116, 40, 49, 55,
            52, 52, 57, 56, 55, 51, 48, 50, 46, 57, 53, 57, 58, 57, 54, 48, 48, 41, 58, 32, 112,
            105, 100, 61, 49, 32, 117, 105, 100, 61, 48, 32, 97, 117, 105, 100, 61, 52, 50, 57, 52,
            57, 54, 55, 50, 57, 53, 32, 115, 101, 115, 61, 52, 50, 57, 52, 57, 54, 55, 50, 57, 53,
            32, 115, 117, 98, 106, 61, 115, 121, 115, 116, 101, 109, 95, 117, 58, 115, 121, 115,
            116, 101, 109, 95, 114, 58, 105, 110, 105, 116, 95, 116, 58, 115, 48, 32, 109, 115,
            103, 61, 39, 117, 110, 105, 116, 61, 78, 101, 116, 119, 111, 114, 107, 77, 97, 110, 97,
            103, 101, 114, 45, 100, 105, 115, 112, 97, 116, 99, 104, 101, 114, 32, 99, 111, 109,
            109, 61, 34, 115, 121, 115, 116, 101, 109, 100, 34, 32, 101, 120, 101, 61, 34, 47, 117,
            115, 114, 47, 108, 105, 98, 47, 115, 121, 115, 116, 101, 109, 100, 47, 115, 121, 115,
            116, 101, 109, 100, 34, 32, 104, 111, 115, 116, 110, 97, 109, 101, 61, 63, 32, 97, 100,
            100, 114, 61, 63, 32, 116, 101, 114, 109, 105, 110, 97, 108, 61, 63, 32, 114, 101, 115,
            61, 115, 117, 99, 99, 101, 115, 115, 39,
        ]);
        let mut reader = LengthDelimitedCodec::builder()
            .native_endian()
            .length_field_type::<u32>()
            .num_skip(0)
            .new_read(reader);

        if let Some(Ok(buf)) = reader.next().await {
            assert_eq!(buf.len(), 243);
        } else {
            panic!()
        }

        if let Some(Ok(buf)) = reader.next().await {
            assert_eq!(buf.len(), 243);
        } else {
            panic!()
        }
    }
}
