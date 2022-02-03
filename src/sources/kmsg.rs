/// Collect messages from /dev/kmsg
///
/// https://www.kernel.org/doc/Documentation/ABI/testing/dev-kmsg
use std::{
    io::{self, Read},
    time,
};

use chrono::{TimeZone, Utc};
use event::{fields, LogRecord};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::config::Output;
use crate::{
    config::{DataType, GenerateConfig, SourceConfig, SourceContext, SourceDescription},
    sources::Source,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct KmsgConfig {}

impl GenerateConfig for KmsgConfig {
    fn generate_config() -> String {
        r#"# No more config option is needed
{}"#
        .into()
    }
}

inventory::submit! {
    SourceDescription::new::<KmsgConfig>("kmsg")
}

#[async_trait::async_trait]
#[typetag::serde(name = "kmsg")]
impl SourceConfig for KmsgConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let mut shutdown = ctx.shutdown;
        let mut output = ctx.output;
        let boot = boot_time("/proc/uptime")?;

        Ok(Box::pin(async move {
            let f = tokio::fs::File::open("/dev/kmsg").await.unwrap();
            let mut reader = tokio::io::BufReader::new(f);
            let mut buf = [0; 4096];

            loop {
                tokio::select! {
                    _ = &mut shutdown => {
                        return Ok(())
                    }

                    result = reader.read(&mut buf[..]) => {
                        let n = match result {
                            Ok(n) => n,
                            Err(err) => {
                                if err.kind() == io::ErrorKind::BrokenPipe {
                                    continue;
                                }

                                error!(
                                    message = "read /dev/kmsg failed",
                                    %err
                                );

                                return Err(())
                            }
                        };

                        if let Ok((priority, seq, ts, msg)) = parse_line(&buf, n) {
                            let nano_seconds = boot + ts * 1000;
                                let timestamp = Utc.timestamp((nano_seconds / (1000 * 1000 * 1000)) as i64, (nano_seconds % (1000 * 1000 * 1000)) as u32);
                                let timestamp_key = log_schema::log_schema().timestamp_key();
                                let record = LogRecord::from(fields!(
                                        "priority" => priority,
                                        "sequence" => seq,
                                        "message" => msg,
                                        timestamp_key => timestamp
                                    ));

                                output.send(record.into()).await.unwrap();
                        }
                    }
                }
            }
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn source_type(&self) -> &'static str {
        "kmsg"
    }
}

fn parse_line(buf: &[u8], size: usize) -> Result<(u8, u64, u64, String), ()> {
    let priority = buf[0] - b'0';
    if buf[1] != b',' {
        return Err(());
    }

    let mut consumed = 2;
    let mut seq = 0u64;
    for i in consumed..size {
        consumed += 1;
        let c = buf[i];
        if !c.is_ascii_digit() {
            if c != b',' {
                return Err(());
            }

            break;
        }

        seq = seq * 10 + (c - b'0') as u64
    }

    let mut ts = 0u64;
    for i in consumed..size {
        consumed += 1;
        let c = buf[i];
        if !c.is_ascii_digit() {
            if c != b',' {
                return Err(());
            }
            break;
        }

        ts = ts * 10 + (c - b'0') as u64
    }

    // parse flags
    for i in consumed..size {
        consumed += 1;
        let c = buf[i];
        if c == b';' {
            break;
        }
    }

    let msg = buf[consumed..size].to_vec();
    let msg = String::from_utf8(msg).map_err(|_| ())?;

    Ok((priority, seq, ts, msg))
}

fn boot_time(path: &str) -> Result<u64, io::Error> {
    let mut buf = [0u8; 256];
    let mut f = std::fs::File::open(path)?;
    let size = f.read(&mut buf[..])?;
    let mut pos = 0;
    let mut sec = 0u64;
    let mut ms = 0u64;

    // read the seconds part
    for i in pos..size {
        pos += 1;
        let c = buf[i];
        if c == b'.' {
            break;
        }

        if c.is_ascii_digit() {
            sec = sec * 10 + (c - b'0') as u64
        } else {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
    }

    pos += 1;

    // Then the microsecond part
    for i in pos..size {
        pos += 1;
        let c = buf[i];
        if c == b' ' {
            break;
        }

        if c.is_ascii_digit() {
            ms = ms * 10 + (c - b'0') as u64;
        } else {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
    }

    let now = time::SystemTime::now();
    let elapsed = time::Duration::from_micros(ms + sec * 1000 * 1000);
    match now.checked_sub(elapsed) {
        Some(boot) => boot
            .duration_since(time::SystemTime::UNIX_EPOCH)
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))
            .map(|d| d.as_nanos() as u64),

        None => Err(io::Error::from(io::ErrorKind::InvalidData)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::config::test_generate_config::<KmsgConfig>()
    }

    #[test]
    fn test_boot_time() {
        // NOTE: the value returned is not constant, so assert_eq! will not help
        boot_time("tests/fixtures/proc/uptime").unwrap();
    }

    #[test]
    fn test_parse_line() {
        let input = r#"4,334322,8544044980,-;RAX: 0000000000000000 RBX: 0000000000000000 RCX: 0000000000000007"#.as_bytes();
        let (priority, seq, ts, msg) = parse_line(input, input.len()).unwrap();

        assert_eq!(priority, 4);
        assert_eq!(seq, 334322);
        assert_eq!(ts, 8544044980);
        assert_eq!(
            msg,
            r#"RAX: 0000000000000000 RBX: 0000000000000000 RCX: 0000000000000007"#
        )
    }
}
