use std::collections::{BTreeMap, HashSet};
use std::io::SeekFrom;
use std::path::Path;
use std::process::Stdio;
use std::{cmp, io, path::PathBuf, time::Duration};

use bytes::{Buf, BytesMut};
use chrono::{DateTime, Utc};
use configurable::configurable_component;
use event::{log::Value, Event};
use framework::config::{DataType, Output, SourceConfig, SourceContext};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::Source;
use futures::{stream::BoxStream, StreamExt};
use log_schema::log_schema;
use nix::{
    sys::signal::{kill, Signal},
    unistd::Pid,
};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::{process::Command, time::sleep};
use tokio_util::codec::{Decoder, FramedRead};

const DEFAULT_BATCH_SIZE: usize = 16;
const CHECKPOINT_FILENAME: &str = "checkpoint.txt";
const CURSOR: &str = "__CURSOR";
const HOSTNAME: &str = "_HOSTNAME";
const MESSAGE: &str = "MESSAGE";
const SYSTEMD_UNIT: &str = "_SYSTEMD_UNIT";
const SOURCE_TIMESTAMP: &str = "_SOURCE_REALTIME_TIMESTAMP";
const RECEIVED_TIMESTAMP: &str = "__REALTIME_TIMESTAMP";

const BACKOFF_DURATION: Duration = Duration::from_secs(1);
const JOURNALCTL: &str = "journalctl";

/// Journald read logs from `journalctl -o export -f -c cursor`
///
/// The format is simple enough to write a parser for it. Without
/// Deserialize(vertex) or Serialize(journalctl) we should get a better
/// performance.
///
/// This source requires permissions to run `journalctl`.
#[configurable_component(source, name = "journald")]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Config {
    /// Only include entries that occurred after the current boot of the system.
    pub current_boot_only: Option<bool>,

    /// A list of unit names to monitor. If empty or not present, all units are accepted.
    /// Unit names lacking a `.` have `.service` appended to make them a valid service
    /// unit name.
    pub units: Vec<String>,

    /// The list of unit names to exclude from monitoring. Unit names lacking a "." will have
    /// ".service" appended to make them a valid service unit name.
    pub excludes: Vec<String>,

    /// The systemd journal is read in batches, and a checkpoint is set at the end of each batch.
    /// This option limits the size of the batch.
    pub batch_size: Option<usize>,

    /// The absolutely path of the `journalctl` executable. If not set, a search is done for
    /// the journalctl path.
    pub journalctl_path: Option<PathBuf>,

    /// The absolutely path of the journal directory. If not set, `journalctl` uses the
    /// default system journal path.
    pub journal_directory: Option<PathBuf>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "journald")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let data_dir = cx.globals.make_subdir(cx.key.id()).map_err(|err| {
            warn!("create sub dir failed {:?}", err);
            err
        })?;

        let includes = self
            .units
            .iter()
            .map(|s| fixup_unit(s))
            .collect::<HashSet<_>>();
        let excludes = self
            .excludes
            .iter()
            .map(|s| fixup_unit(s))
            .collect::<HashSet<_>>();

        let checkpointer = Checkpointer::new(data_dir.join(CHECKPOINT_FILENAME)).await?;
        let journalctl_path = self
            .journalctl_path
            .clone()
            .unwrap_or_else(|| JOURNALCTL.into());
        let journal_dir = self.journal_directory.clone();
        let current_boot_only = self.current_boot_only.unwrap_or(true);

        let src = JournaldSource {
            includes,
            excludes,
            batch_size: self.batch_size.unwrap_or(DEFAULT_BATCH_SIZE),
            output: cx.output,
        };

        let start: StartJournalctlFn = Box::new(move |cursor| {
            let mut command = create_command(
                &journalctl_path,
                journal_dir.as_ref(),
                current_boot_only,
                cursor,
            );

            start_journalctl(&mut command)
        });

        Ok(Box::pin(src.run_shutdown(checkpointer, cx.shutdown, start)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

/// Map the given unit name into a valid systemd unit
/// by appending ".service" if no extension is present
fn fixup_unit(unit: &str) -> String {
    if unit.contains('.') {
        unit.into()
    } else {
        format!("{}.service", unit)
    }
}

struct JournaldSource {
    includes: HashSet<String>,
    excludes: HashSet<String>,
    batch_size: usize,
    output: Pipeline,
}

impl JournaldSource {
    async fn run_shutdown(
        self,
        mut checkpointer: Checkpointer,
        shutdown: ShutdownSignal,
        start: StartJournalctlFn,
    ) -> Result<(), ()> {
        let mut cursor = match checkpointer.get().await {
            Ok(cursor) => cursor,
            Err(err) => {
                error!(
                    message = "Could not retrieve saved journald checkpoint",
                    %err
                );
                None
            }
        };

        let mut on_stop = None;
        let run = Box::pin(self.run(&mut checkpointer, &mut cursor, &mut on_stop, start));

        info!("start selecting");
        futures::future::select(run, shutdown).await;
        info!("stopping journal");
        if let Some(stop) = on_stop {
            stop();
        }

        Self::save_checkpoint(&mut checkpointer, &cursor).await;

        Ok(())
    }

    async fn run<'a>(
        mut self,
        checkpointer: &'a mut Checkpointer,
        cursor: &'a mut Option<String>,
        on_stop: &'a mut Option<StopJournalctlFn>,
        start: StartJournalctlFn,
    ) {
        loop {
            info!("starting journalctl");
            match start(&*cursor) {
                Ok((stream, stop)) => {
                    *on_stop = Some(stop);
                    let should_restart = self.run_stream(stream, checkpointer, cursor).await;
                    if let Some(stop) = on_stop.take() {
                        stop();
                    }

                    if !should_restart {
                        return;
                    }
                }

                Err(err) => {
                    error!(
                        message = "Error starting journalctl process",
                        %err
                    );
                }
            };

            // journalctl process should never stop,
            // so it is an error if we reach here
            sleep(BACKOFF_DURATION).await;
        }
    }

    /// Process `journalctl` output until some error occurs.
    /// Return `true` if should restart `journalctl`
    async fn run_stream<'a>(
        &'a mut self,
        mut stream: BoxStream<'static, Result<BTreeMap<String, Value>, io::Error>>,
        checkpointer: &'a mut Checkpointer,
        cursor: &'a mut Option<String>,
    ) -> bool {
        loop {
            let mut saw_record = false;

            for _ in 0..self.batch_size {
                let mut entry = match stream.next().await {
                    None => {
                        warn!("journalctl process stopped");
                        return true;
                    }
                    Some(Ok(entry)) => entry,
                    Some(Err(err)) => {
                        error!(
                            message = "Could not read from journald source",
                            %err
                        );

                        break;
                    }
                };

                if let Some(tmp) = entry.remove(CURSOR) {
                    if let Value::Bytes(_) = tmp {
                        *cursor = Some(tmp.to_string_lossy());
                    }
                }

                saw_record = true;
                if let Some(Value::Bytes(value)) = entry.get(SYSTEMD_UNIT) {
                    let s = String::from_utf8_lossy(value);
                    if self.filter(&s) {
                        continue;
                    }
                }

                match self.output.send(create_event(entry)).await {
                    Ok(_) => {}
                    Err(err) => {
                        error!(
                            message = "Could not send journald log",
                            %err
                        );

                        // output channel is closed, don't restart journalctl
                        return false;
                    }
                }
            }

            if saw_record {
                Self::save_checkpoint(checkpointer, cursor).await;
            }
        }
    }

    fn filter(&self, unit: &str) -> bool {
        if self.excludes.contains(unit) {
            return true;
        }

        if self.includes.is_empty() {
            return false;
        }

        !self.includes.contains(unit)
    }

    async fn save_checkpoint(checkpointer: &mut Checkpointer, cursor: &Option<String>) {
        if let Some(cursor) = cursor {
            if let Err(err) = checkpointer.set(cursor).await {
                error!(
                    message = "Could not set journald checkpoint",
                    %err,
                    filename = ?checkpointer.path,
                );
            }
        }
    }
}

fn create_event(entry: BTreeMap<String, Value>) -> Event {
    let mut log: event::LogRecord = entry.into();

    // Convert some journald-specific field names into LogSchema's
    if let Some(msg) = log.remove_field(MESSAGE) {
        log.insert_field(log_schema().message_key(), msg);
    }
    if let Some(host) = log.remove_field(HOSTNAME) {
        log.insert_field(log_schema().host_key(), host);
    }
    // Translate the timestamp, and so leave both old and new names
    if let Some(Value::Bytes(timestamp)) = log
        .get_field(SOURCE_TIMESTAMP)
        .or_else(|| log.get_field(RECEIVED_TIMESTAMP))
    {
        if let Ok(timestamp) = String::from_utf8_lossy(timestamp).parse::<u64>() {
            let timestamp = DateTime::<Utc>::from_timestamp(
                (timestamp / 1_000_000) as i64,
                (timestamp % 1_000_000) as u32 * 1_000,
            )
            .expect("valid timestamp");

            log.insert_field(log_schema().timestamp_key(), Value::Timestamp(timestamp));
        }
    }

    // Add source type
    log.insert_field(log_schema().source_type_key(), "journald");

    log.into()
}

/// A function that starts journalctl process.
/// Return a stream of output splitted by '\n\n', and a `StopJournalctlFn`,
///
/// Code uses `start_journalctl` below,
/// but we need this type to implement fake journald source in testing
type StartJournalctlFn = Box<
    dyn Fn(
            &Option<String>,
        ) -> crate::Result<(
            BoxStream<'static, Result<BTreeMap<String, Value>, io::Error>>,
            StopJournalctlFn,
        )> + Send
        + Sync,
>;

type StopJournalctlFn = Box<dyn FnOnce() + Send>;

fn create_command(
    path: &Path,
    journal_dir: Option<&PathBuf>,
    current_boot_only: bool,
    cursor: &Option<String>,
) -> Command {
    let mut command = Command::new(path);
    command
        .stdout(Stdio::piped())
        .arg("--follow")
        .arg("--output=export");

    if let Some(dir) = journal_dir {
        command.arg(format!("--directory={}", dir.display()));
    }

    if current_boot_only {
        command.arg("--boot");
    }

    if let Some(cursor) = cursor {
        command.arg(format!("--after-cursor={}", cursor));
    } else {
        // journalctl --follow only outputs a few lines without a starting point
        command.arg("--since=2000-01-01");
    }

    command
}

fn start_journalctl(
    command: &mut Command,
) -> crate::Result<(
    BoxStream<'static, Result<BTreeMap<String, Value>, io::Error>>,
    StopJournalctlFn,
)> {
    let mut child = command.spawn()?;
    let stream = FramedRead::new(child.stdout.take().unwrap(), EntryCodec::new()).boxed();

    let pid = Pid::from_raw(child.id().unwrap() as _);
    let stop = Box::new(move || {
        let _ = kill(pid, Signal::SIGTERM);
    });

    Ok((stream, stop))
}

struct Checkpointer {
    file: tokio::fs::File,
    path: PathBuf,
}

impl Checkpointer {
    async fn new(path: PathBuf) -> Result<Self, io::Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .await?;

        Ok(Self { file, path })
    }

    async fn set(&mut self, token: &str) -> Result<(), io::Error> {
        self.file.seek(SeekFrom::Start(0)).await?;
        self.file.write_all(token.as_bytes()).await?;
        Ok(())
    }

    async fn get(&mut self) -> Result<Option<String>, io::Error> {
        let mut buf = Vec::<u8>::new();
        self.file.seek(SeekFrom::Start(0)).await?;
        self.file.read_to_end(&mut buf).await?;
        match buf.len() {
            0 => Ok(None),
            _ => {
                let text = String::from_utf8_lossy(&buf);
                Ok(Some(String::from(text)))
            }
        }
    }
}

/// Codec for Journal Export format
/// https://www.freedesktop.org/wiki/Software/systemd/export/
struct EntryCodec {
    max_length: usize,
    discarding: bool,
    next: usize,

    // mid state for decode
    fields: BTreeMap<String, Value>,
}

impl EntryCodec {
    fn new() -> Self {
        Self {
            max_length: 16 * 1024,
            discarding: false,
            next: 0,
            fields: Default::default(),
        }
    }
}

fn decode_kv(buf: &[u8]) -> Result<(String, Value), io::Error> {
    let mut pos = 0;
    let length = buf.len();

    while pos < length {
        if buf[pos] == b'=' {
            break;
        }

        pos += 1;
    }

    if pos == length || pos + 1 == length {
        return Err(io::Error::from(io::ErrorKind::InvalidData));
    }

    let key = String::from_utf8(buf[0..pos].to_vec())
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    // +1 to skip '='
    pos += 1;

    return match key.as_ref() {
        "PRIORITY" => {
            let value = match (buf[length - 1] - b'0') as i32 {
                0 => "EMERG",
                1 => "ALERT",
                2 => "CRIT",
                3 => "ERR",
                4 => "WARNING",
                5 => "NOTICE",
                6 => "INFO",
                7 => "DEBUG",
                _ => "UNKNOWN",
            };
            Ok((key, value.into()))
        }
        "_SOURCE_MONOTONIC_TIMESTAMP" => {
            let mut ts = 0u64;
            while pos < length {
                let c = buf[pos];
                if c.is_ascii_digit() {
                    ts = ts * 10 + (c - b'0') as u64
                } else {
                    break;
                }

                pos += 1;
            }

            Ok((key, ts.into()))
        }
        _ => {
            let value = String::from_utf8(buf[pos..length].to_vec())
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            if key == MESSAGE {
                Ok(("message".to_string(), value.into()))
            } else {
                Ok((key, value.into()))
            }
        }
    };
}

impl Decoder for EntryCodec {
    type Item = BTreeMap<String, Value>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            //
            let read_to = cmp::min(self.max_length, buf.len());
            let newline_pos = buf[0..read_to].iter().position(|b| *b == b'\n');

            match (self.discarding, newline_pos) {
                (true, Some(offset)) => {
                    // If we found a newline, discard up to that offset and then stop discarding.
                    // On the next iteration, we'll try to read a line normally
                    buf.advance(offset + self.next + 1);
                    self.next = 0;
                    self.discarding = false;
                }

                (true, None) => {
                    // Otherwise, we didn't find a newline, so we'll discard
                    // everything we read. On the next iteration, we'll continue
                    // discarding up to max_len bytes unless we find a newline
                    buf.advance(read_to);
                    self.next = 0;
                    if buf.is_empty() {
                        return Ok(None);
                    }
                }

                (false, Some(offset)) => {
                    // we found a correct frame
                    if offset == 0 {
                        // new frame
                        buf.advance(1);
                        let fields = self.fields.clone();
                        self.fields.clear();
                        return Ok(Some(fields));
                    }

                    self.next = 0;

                    if let Ok((k, v)) = decode_kv(&buf[self.next..offset]) {
                        // +1 for the \n
                        buf.advance(offset + 1);
                        self.fields.insert(k, v);
                    } else {
                        self.discarding = true
                    }
                }

                (false, None) => {
                    if buf.len() > self.max_length {
                        // We reached the max length without finding the delimiter,
                        // so must discard the rest until we reach the next delimiter
                        self.discarding = true;
                        return Ok(None);
                    }

                    // We didn't find the delimiter and didn't reach the max length
                    self.next = read_to;
                    return Ok(None);
                }
            }
        }
    }
}

#[cfg(test)]
mod checkpoints_tests {
    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn test_checkpoints() {
        let tempdir = tempdir().unwrap();
        let mut filename = tempdir.path().to_path_buf();
        filename.push(CHECKPOINT_FILENAME);
        let mut checkpointer = Checkpointer::new(filename).await.unwrap();

        // read nothing
        assert!(checkpointer.get().await.unwrap().is_none());

        // read first write
        checkpointer.set("foo").await.unwrap();
        assert_eq!(checkpointer.get().await.unwrap(), Some("foo".to_string()));

        // read more
        checkpointer.set("bar").await.unwrap();
        assert_eq!(checkpointer.get().await.unwrap(), Some("bar".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use chrono::TimeZone;
    use event::Event;
    use tempfile::tempdir;
    use tokio::time::{sleep, timeout, Duration};
    use tokio_stream::StreamExt;

    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }

    #[test]
    fn test_decode_kv() {
        let (k, v) = decode_kv("_SYSTEMD_UNIT=NetworkManager.service".as_bytes()).unwrap();
        assert_eq!(k, "_SYSTEMD_UNIT");
        assert_eq!(v, Value::Bytes("NetworkManager.service".into()));

        let (k, v) = decode_kv("PRIORITY=5".as_bytes()).unwrap();
        assert_eq!(k, "PRIORITY");
        assert_eq!(v, Value::Bytes("NOTICE".into()));

        // unknown priority
        let (k, v) = decode_kv("PRIORITY=9".as_bytes()).unwrap();
        assert_eq!(k, "PRIORITY");
        assert_eq!(v, Value::Bytes("UNKNOWN".into()));

        // decode timestamp
        let (k, v) = decode_kv("_SOURCE_REALTIME_TIMESTAMP=1578529839140003".as_bytes()).unwrap();
        assert_eq!(k, "_SOURCE_REALTIME_TIMESTAMP");
        assert_eq!(v, Value::Bytes("1578529839140003".into()));

        // no `=`
        assert!(decode_kv("foo".as_bytes()).is_err());

        // no value
        assert!(decode_kv("foo=".as_bytes()).is_err());
    }

    #[tokio::test]
    async fn test_codec() {
        let reader = tokio::fs::File::open("tests/fixtures/journal_export.txt")
            .await
            .unwrap();
        let mut stream = FramedRead::new(reader, EntryCodec::new());

        let mut count = 0;
        while let Some(result) = stream.next().await {
            if result.is_ok() {
                count += 1;
            }
        }

        assert_eq!(count, 8);
    }

    struct TestJournal {}

    impl TestJournal {
        fn stream(
            _checkpoint: &Option<String>,
        ) -> (
            BoxStream<'static, Result<BTreeMap<String, Value>, io::Error>>,
            StopJournalctlFn,
        ) {
            (journal_stream(), Box::new(|| ()))
        }
    }

    fn journal_stream() -> BoxStream<'static, Result<BTreeMap<String, Value>, io::Error>> {
        let text = r#"_SYSTEMD_UNIT=sysinit.target
MESSAGE=System Initialization
__CURSOR=1
_SOURCE_REALTIME_TIMESTAMP=1578529839140001
PRIORITY=6

_SYSTEMD_UNIT=unit.service
MESSAGE=unit message
__CURSOR=2
_SOURCE_REALTIME_TIMESTAMP=1578529839140002
PRIORITY=7

_SYSTEMD_UNIT=badunit.service
MESSAGE=[194,191,72,101,108,108,111,63]
__CURSOR=2
_SOURCE_REALTIME_TIMESTAMP=1578529839140003
PRIORITY=5

_SYSTEMD_UNIT=stdout
MESSAGE=Missing timestamp
__CURSOR=3
__REALTIME_TIMESTAMP=1578529839140004
PRIORITY=2

_SYSTEMD_UNIT=stdout
MESSAGE=Different timestamps
__CURSOR=4
_SOURCE_REALTIME_TIMESTAMP=1578529839140005
__REALTIME_TIMESTAMP=1578529839140004
PRIORITY=3

_SYSTEMD_UNIT=syslog.service
MESSAGE=Non-ASCII in other field
__CURSOR=5
_SOURCE_REALTIME_TIMESTAMP=1578529839140005
__REALTIME_TIMESTAMP=1578529839140004
PRIORITY=3
SYSLOG_RAW=[194,191,87,111,114,108,100,63]

_SYSTEMD_UNIT=NetworkManager.service
MESSAGE=<info>  [1608278027.6016] dhcp-init: Using DHCP client 'dhclient'
__CURSOR=6
_SOURCE_REALTIME_TIMESTAMP=1578529839140005
__REALTIME_TIMESTAMP=1578529839140004
PRIORITY=6
SYSLOG_FACILITY=[DHCP4, DHCP6]

PRIORITY=5
SYSLOG_FACILITY=0
SYSLOG_IDENTIFIER=kernel
_TRANSPORT=kernel
__REALTIME_TIMESTAMP=1578529839140006
MESSAGE=audit log

"#;
        let cursor = Cursor::new(text);
        let reader = tokio::io::BufReader::new(cursor);
        let stream = FramedRead::new(reader, EntryCodec::new());

        Box::pin(stream)
    }

    #[tokio::test]
    async fn test_journal_stream() {
        let mut count = 0;
        let mut stream = journal_stream();
        while let Some(result) = stream.next().await {
            result.unwrap();
            count += 1;
        }

        assert_eq!(count, 8)
    }

    fn create_unit_matches<S: Into<String>>(units: Vec<S>) -> HashSet<String> {
        let units: HashSet<String> = units.into_iter().map(Into::into).collect();
        units
    }

    async fn run_with_units(
        includes: &[&str],
        excludes: &[&str],
        cursor: Option<&str>,
    ) -> Vec<Event> {
        let includes = create_unit_matches(includes.to_vec());
        let excludes = create_unit_matches(excludes.to_vec());
        run_journal(includes, excludes, cursor).await
    }

    async fn run_journal(
        includes: HashSet<String>,
        excludes: HashSet<String>,
        cursor: Option<&str>,
    ) -> Vec<Event> {
        let (tx, rx) = Pipeline::new_test();
        let (trigger, shutdown, _) = ShutdownSignal::new_wired();
        let tempdir = tempdir().unwrap();
        let mut checkpoint_path = tempdir.path().to_path_buf();
        checkpoint_path.push(CHECKPOINT_FILENAME);

        let mut checkpointer = Checkpointer::new(checkpoint_path.clone())
            .await
            .expect("Creating checkpointer failed");
        if let Some(cursor) = cursor {
            checkpointer
                .set(cursor)
                .await
                .expect("Could not set checkpoint");
        }

        let source = JournaldSource {
            includes,
            excludes,
            batch_size: DEFAULT_BATCH_SIZE,
            output: tx,
        };

        tokio::spawn(source.run_shutdown(
            checkpointer,
            shutdown,
            Box::new(|checkpoint| Ok(TestJournal::stream(checkpoint))),
        ));

        sleep(Duration::from_millis(200)).await;
        drop(trigger);
        timeout(Duration::from_secs(1), rx.collect::<Vec<Event>>())
            .await
            .unwrap()
    }

    fn test_journal(includes: &[&str], excludes: &[&str]) -> JournaldSource {
        let (tx, _) = Pipeline::new_test();
        JournaldSource {
            includes: create_unit_matches(includes.to_vec()),
            excludes: create_unit_matches(excludes.to_vec()),
            batch_size: DEFAULT_BATCH_SIZE,
            output: tx,
        }
    }

    #[test]
    fn unit_filter() {
        // if nothing configured, allow all
        let journal = test_journal(&[], &[]);
        assert!(!journal.filter("foo"));
        assert!(!journal.filter("bar"));

        // filter one
        let journal = test_journal(&[], &["foo"]);
        assert!(journal.filter("foo"));
        assert!(!journal.filter("bar"));
    }

    #[tokio::test]
    async fn reads_journal() {
        let received = run_with_units(&[], &[], None).await;
        assert_eq!(received.len(), 8);
        assert_eq!(
            message(&received[0]),
            Value::Bytes("System Initialization".into())
        );

        assert_eq!(timestamp(&received[0]), value_ts(1578529839, 140001000));
        assert_eq!(priority(&received[0]), Value::Bytes("INFO".into()));
        assert_eq!(message(&received[1]), Value::Bytes("unit message".into()));
        assert_eq!(timestamp(&received[1]), value_ts(1578529839, 140002000));
        assert_eq!(priority(&received[1]), Value::Bytes("DEBUG".into()));
    }

    fn message(event: &Event) -> Value {
        let log = event.as_log();
        let v = log.fields.get("message").unwrap();
        match v {
            Value::Bytes(_) => v.clone(),
            _ => panic!("invalid event"),
        }
    }

    fn value_ts(secs: i64, usecs: u32) -> Value {
        Value::Timestamp(DateTime::<Utc>::from_timestamp(secs, usecs).unwrap())
    }

    fn timestamp(event: &Event) -> Value {
        let log = event.as_log();
        let v = log.fields.get("_SOURCE_REALTIME_TIMESTAMP").unwrap();
        let ns = match v {
            Value::Bytes(s) => {
                let s = String::from_utf8_lossy(s);
                s.parse::<i64>().unwrap()
            }
            _ => panic!("unexpected timestamp type"),
        };

        Value::Timestamp(Utc.timestamp_nanos(ns * 1000))
    }

    fn priority(event: &Event) -> Value {
        let log = event.as_log();
        let v = log.fields.get("PRIORITY").unwrap();
        match v {
            Value::Bytes(_) => v.clone(),
            _ => panic!("invalid event"),
        }
    }
}
