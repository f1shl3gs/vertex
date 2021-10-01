/// Collect messages from /dev/kmsg
///
/// https://www.kernel.org/doc/Documentation/ABI/testing/dev-kmsg

use std::{
    io::{
        self, Read,
    },
    time,
};
use futures::SinkExt;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use crate::{
    fields,
    sources::Source,
    event::LogRecord,
    config::{DataType, SourceConfig, SourceContext},
};

#[derive(Debug, Deserialize, Serialize)]
struct KmsgConfig {}

#[async_trait::async_trait]
#[typetag::serde(name = "kmsg")]
impl SourceConfig for KmsgConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let mut shutdown = ctx.shutdown;
        let mut output = ctx.out;
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
                                    "read /dev/kmsg failed";
                                    "err" => err
                                );

                                return Err(())
                            }
                        };

                        match parse_line(&buf, n) {
                            Ok((priority, seq, ts, msg)) => {
                                let record = LogRecord {
                                    time_unix_nano: boot + ts * 1000,
                                    tags: Default::default(),
                                    fields: fields!(
                                        "priority" => priority,
                                        "sequence" => seq,
                                        "message" => msg
                                    ),
                                };

                                output.send(record.into()).await.unwrap();
                            }

                            _ => {}
                        }
                    }
                }
            }
        }))
    }

    fn output_type(&self) -> DataType {
        DataType::Log
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
        if c < b'0' || c > b'9' {
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
        if c < b'0' || c > b'9' {
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
    let msg = String::from_utf8(msg)
        .map_err(|_| ())?;

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
        Some(boot) => boot.duration_since(time::SystemTime::UNIX_EPOCH)
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))
            .map(|d| d.as_nanos() as u64),

        None => Err(io::Error::from(io::ErrorKind::InvalidData))
    }
}

#[cfg(test)]
mod tests {
    use std::io::{stdout, Write};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt};
    use super::*;

    #[test]
    fn test_boot_time() {
        let ns = boot_time("/proc/uptime").unwrap();
        println!("{}", ns);
    }

    #[tokio::test]
    async fn test_read() {
        let f = tokio::fs::File::open("/dev/kmsg").await.unwrap();
        let mut reader = tokio::io::BufReader::new(f);

        let mut buf = [0; 256];
        let n = reader.read(&mut buf[..]).await.unwrap();
        let v = buf[..n].to_vec();
        println!("{}", String::from_utf8(v).unwrap());

        let mut buf = [0; 256];
        let n = reader.read(&mut buf[..]).await.unwrap();
        let v = buf[..n].to_vec();
        println!("{}", String::from_utf8(v).unwrap());

        let mut buf = [0; 256];
        let n = reader.read(&mut buf[..]).await.unwrap();
        let v = buf[..n].to_vec();
        println!("{}", String::from_utf8(v).unwrap());
    }

    #[test]
    fn test_parse_line() {
        let input = r#"4,334322,8544044980,-;RAX: 0000000000000000 RBX: 0000000000000000 RCX: 0000000000000007"#.as_bytes();
        let (priority, seq, ts, msg) = parse_line(input, input.len()).unwrap();

        assert_eq!(priority, 4);
        assert_eq!(seq, 334322);
        assert_eq!(ts, 8544044980);
        assert_eq!(msg, r#"RAX: 0000000000000000 RBX: 0000000000000000 RCX: 0000000000000007"#)
    }
}