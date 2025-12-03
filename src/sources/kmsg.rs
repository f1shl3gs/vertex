//! Collect messages from /dev/kmsg
//!
//! https://www.kernel.org/doc/Documentation/ABI/testing/dev-kmsg

use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::FileExt;
use std::path::Path;
use std::time;

use chrono::{DateTime, Utc};
use configurable::configurable_component;
use event::LogRecord;
use framework::Source;
use framework::config::{OutputType, SourceConfig, SourceContext};
use tokio::io::AsyncBufReadExt;
use value::value;

#[configurable_component(source, name = "kmsg")]
#[serde(deny_unknown_fields)]
struct Config {}

#[async_trait::async_trait]
#[typetag::serde(name = "kmsg")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let f = tokio::fs::File::open("/dev/kmsg").await?;
        let mut reader = tokio::io::BufReader::new(f);

        let boot = boot_time("/proc/uptime")?;

        let path = cx.globals.make_subdir(cx.key.id())?;
        let mut checkpointer = Checkpointer::new(path.join("checkpoint.data"))?;
        let mut last_sent = checkpointer.load()?;

        let mut shutdown = cx.shutdown;
        let mut output = cx.output;

        Ok(Box::pin(async move {
            let mut buf = String::new();

            loop {
                buf.clear();
                tokio::select! {
                    _ = &mut shutdown => break,
                    result = reader.read_line(&mut buf) => if let Err(err) = result {
                        warn!(
                            message = "Error reading from /dev/kmsg",
                            ?err,
                            internal_log_rate_secs = 30
                        );

                        continue;
                    }
                }

                if let Ok((priority, seq, ts, msg)) = parse_line(buf.as_bytes()) {
                    let nano_seconds = boot + ts * 1000;
                    if nano_seconds <= last_sent {
                        continue;
                    }

                    let timestamp = DateTime::<Utc>::from_timestamp(
                        (nano_seconds / (1000 * 1000 * 1000)) as i64,
                        (nano_seconds % (1000 * 1000 * 1000)) as u32,
                    )
                    .unwrap();
                    let record = LogRecord::from(value!({
                        "priority": priority,
                        "sequence": seq,
                        "message": msg,
                        "timestamp": timestamp
                    }));

                    if let Err(_err) = output.send(record).await {
                        warn!(message = "Error while sending kmsg log record",);

                        break;
                    }

                    last_sent = nano_seconds;
                    if let Err(err) = checkpointer.set(last_sent) {
                        warn!(
                            message = "Error while save checkpoint for kmsg",
                            ?err,
                            internal_log_rate_secs = 30
                        );
                    }
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::log()]
    }
}

fn parse_line(buf: &[u8]) -> Result<(u8, u64, u64, String), ()> {
    let priority = buf[0] - b'0';
    if buf[1] != b',' {
        return Err(());
    }

    let mut consumed = 2;
    let mut seq = 0u64;
    while consumed < buf.len() {
        let c = buf[consumed];
        consumed += 1;
        if !c.is_ascii_digit() {
            if c != b',' {
                return Err(());
            }

            break;
        }

        seq = seq * 10 + (c - b'0') as u64;
    }

    let mut ts = 0u64;
    while consumed < buf.len() {
        let c = buf[consumed];
        consumed += 1;
        if !c.is_ascii_digit() {
            if c != b',' {
                return Err(());
            }

            break;
        }

        ts = ts * 10 + (c - b'0') as u64;
    }

    // parse flags
    while consumed < buf.len() {
        let c = buf[consumed];
        consumed += 1;

        if c == b';' {
            break;
        }
    }

    let msg = buf[consumed..].to_vec();
    let msg = String::from_utf8(msg).map_err(|_| ())?;

    Ok((priority, seq, ts, msg))
}

/// https://man7.org/linux/man-pages/man5/proc_uptime.5.html
fn boot_time(path: &str) -> Result<u64, io::Error> {
    let mut buf = [0u8; 256];
    let mut f = File::open(path)?;
    let size = f.read(&mut buf[..])?;
    let mut pos = 0;
    let mut sec = 0u64;
    let mut ms = 0u64;

    // read the seconds part
    while pos < size {
        let c = buf[pos];
        pos += 1;
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
    while pos < size {
        let c = buf[pos];
        pos += 1;
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

struct Checkpointer {
    file: File,
}

impl Checkpointer {
    fn new(path: impl AsRef<Path>) -> io::Result<Checkpointer> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;

        Ok(Checkpointer { file })
    }

    fn load(&mut self) -> io::Result<u64> {
        let mut buf = [0u8; size_of::<u64>()];

        match self.file.read_exact_at(&mut buf, 0) {
            Ok(_) => Ok(u64::from_ne_bytes(buf)),
            Err(err) => {
                if err.kind() == io::ErrorKind::UnexpectedEof {
                    return Ok(0);
                }

                Err(err)
            }
        }
    }

    #[inline]
    fn set(&mut self, timestamp: u64) -> io::Result<()> {
        self.file.write_all_at(&timestamp.to_ne_bytes(), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }

    #[test]
    fn test_boot_time() {
        // NOTE: the value returned is not constant, so assert_eq! will not help
        boot_time("tests/node/proc/uptime").unwrap();
    }

    #[test]
    fn test_parse_line() {
        let input = r#"4,334322,8544044980,-;RAX: 0000000000000000 RBX: 0000000000000000 RCX: 0000000000000007"#.as_bytes();
        let (priority, seq, ts, msg) = parse_line(input).unwrap();

        assert_eq!(priority, 4);
        assert_eq!(seq, 334322);
        assert_eq!(ts, 8544044980);
        assert_eq!(
            msg,
            r#"RAX: 0000000000000000 RBX: 0000000000000000 RCX: 0000000000000007"#
        )
    }
}
