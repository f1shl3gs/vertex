/// Journald read logs from `journalctl -o export -f -c cursor`
///
/// The format is simple enough to write a parser for it. Without
/// Deserialize(vertex) or Serialize(journalctl) we should get a better
/// performance.
///
/// ```text
/// __CURSOR=s=927da5db44c94c348d4974923a29eadb;i=443072;b=092dc5604f20408d8569c10fb41a5107;m=17b2cd25c0;t=5cd4d312cb1bd;x=bfc589b9a9802207
/// __REALTIME_TIMESTAMP=1633106304741821
/// __MONOTONIC_TIMESTAMP=101784036800
/// _BOOT_ID=092dc5604f20408d8569c10fb41a5107
/// _TRANSPORT=kernel
/// PRIORITY=4
/// SYSLOG_FACILITY=0
/// SYSLOG_IDENTIFIER=kernel
/// _MACHINE_ID=ba51360e6b1e423e84e59950b031f7b1
/// _HOSTNAME=localhost.localdomain
/// _SOURCE_MONOTONIC_TIMESTAMP=101784144369
/// MESSAGE=RDX: 0000000000000080 RSI: 000000c000130b80 RDI: 0000000000000009
///
/// __CURSOR=s=927da5db44c94c348d4974923a29eadb;i=443073;b=092dc5604f20408d8569c10fb41a5107;m=17b2cd25cd;t=5cd4d312cb1ca;x=1cb7fa29f9706fad
/// __REALTIME_TIMESTAMP=1633106304741834
/// __MONOTONIC_TIMESTAMP=101784036813
/// _BOOT_ID=092dc5604f20408d8569c10fb41a5107
/// _TRANSPORT=kernel
/// PRIORITY=4
/// SYSLOG_FACILITY=0
/// SYSLOG_IDENTIFIER=kernel
/// _MACHINE_ID=ba51360e6b1e423e84e59950b031f7b1
/// _HOSTNAME=localhost.localdomain
/// MESSAGE=RBP: 000000c00027d7a8 R08: 0000000000b17b20 R09: 0000000000000000
/// _SOURCE_MONOTONIC_TIMESTAMP=101784144369
/// ```

use std::collections::{BTreeMap, HashSet};
use std::{io, fs::File, path::PathBuf, cmp, time::Duration};
use std::os::unix::raw::pid_t;
use std::path::Path;
use std::process::Stdio;
use serde::{Deserialize, Serialize};
use crate::config::{DataType, SourceConfig, SourceContext};
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;
use crate::sources::Source;
use tracing::{error, info};
use tokio::{
    process::Command,
    time::sleep,
};

use tokio_util::codec::{Decoder, FramedRead};
use bytes::{BytesMut, Buf};
use futures::{
    SinkExt,
    StreamExt,
    stream::BoxStream,
};
use crate::event::Value;
use lazy_static::lazy_static;

use nix::{
    sys::signal::{kill, Signal},
    unistd::Pid,
};

const CHECKPOINT_FILENAME: &str = "checkpoint.txt";
const CURSOR: &str = "__CURSOR";
const HOSTNAME: &str = "_HOSTNAME";
const MESSAGE: &str = "MESSAGE";
const SYSTEMD_UNIT: &str = "_SYSTEMD_UNIT";
const SOURCE_TIMESTAMP: &str = "_SOURCE_REALTIME_TIMESTAMP";
const RECEIVED_TIMESTAMP: &str = "__REALTIME_TIMESTAMP";

const BACKOFF_DURATION: Duration = Duration::from_secs(1);

lazy_static! {
    static ref JOURNALCTL: PathBuf = "journalctl".into();
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct JournaldConfig {
    pub current_boot_only: Option<bool>,
    pub units: Vec<String>,
    /// The list of unit names to exclude from monitoring. Unit names lacking a "." will have
    /// ".service" appended to make them a valid service unit name.
    pub excludes: Vec<String>,
    /// The systemd journal is read in batches, and a checkpoint is set at the end of each batch.
    /// This option limits the size of the batch.
    pub batch_size: Option<usize>,
    pub journalctl_path: Option<PathBuf>,
    pub journal_directory: Option<PathBuf>,

}

#[async_trait::async_trait]
#[typetag::serde(name = "journald")]
impl SourceConfig for JournaldConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let data_dir = ctx.global.make_subdir(ctx.name.as_str())
            .map_err(|err| {
                warn!("create sub dir failed {:?}", err);
                err
            })?;

        let includes = self.units
            .iter()
            .map(|s| fixup_unit(s))
            .collect::<HashSet<_>>();
        let excludes = self.excludes
            .iter()
            .map(|s| fixup_unit(s))
            .collect::<HashSet<_>>();

        let checkpointer = Checkpointer::new(data_dir).await?;
        let journalctl_path = self.journalctl_path
            .clone()
            .unwrap_or_else(|| JOURNALCTL.clone());
        let journal_dir = self.journal_directory.clone();
        let current_boot_only = self.current_boot_only.unwrap_or(true);

        let src = JournaldSource {
            includes,
            excludes,
            batch_size: self.batch_size.unwrap_or(0),
            output: ctx.out,
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

        Ok(Box::pin(src.run_shutdown(checkpointer, ctx.shutdown, start)))
    }

    fn output_type(&self) -> DataType {
        DataType::Log
    }

    fn source_type(&self) -> &'static str {
        "journald"
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
        let run = Box::pin(self.run(
            &mut checkpointer,
            &mut cursor,
            &mut on_stop,
            start,
        ));

        futures::future::select(run, shutdown).await;
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

                if let Some(tmp) = entry.remove(&*CURSOR) {
                    match tmp {
                        Value::String(s) => *cursor = Some(s),
                        _ => {}
                    }
                }

                saw_record = true;
                if let Some(tmp) = entry.get(SYSTEMD_UNIT) {
                    match tmp {
                        Value::String(unit) => {
                            if self.filter(unit) {
                                continue;
                            }
                        }
                        _ => {}
                    }
                }

                match self.output.send(entry.into()).await {
                    Ok(_) => {}
                    Err(err) => {
                        error!(
                            message = "Could not send journald log",
                            %err
                        );

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

        !self.includes.contains(unit)
    }

    async fn save_checkpoint(
        checkpointer: &mut Checkpointer,
        cursor: &Option<String>,
    ) {
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
    )> + Send + Sync,
>;

type StopJournalctlFn = Box<dyn FnOnce() + Send>;

fn create_command(
    path: &Path,
    journal_dir: Option<&PathBuf>,
    current_boot_only: bool,
    cursor: &Option<String>,
) -> Command {
    let mut command = Command::new(path);
    command.stdout(Stdio::piped())
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
    let stream = FramedRead::new(
        child.stdout.take().unwrap(),
        EntryCodec::new(),
    ).boxed();

    let pid = Pid::from_raw(child.id().unwrap() as i32 as pid_t);
    let stop = Box::new(move || {
        let _ = kill(pid, Signal::SIGTERM);
    });

    Ok((stream, stop))
}

struct Checkpointer {
    file: File,
    path: PathBuf,
}

impl Checkpointer {
    async fn new(path: PathBuf) -> Result<Self, io::Error> {
        todo!()
    }

    async fn set(&mut self, token: &str) -> Result<(), io::Error> {
        todo!()
    }

    async fn get(&mut self) -> Result<Option<String>, io::Error> {
        todo!()
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
    for i in pos..length {
        pos += 1;
        let c = buf[i];
        if c == b'=' {
            break;
        }
    }
    let key = String::from_utf8(buf[0..pos].to_vec())
        .map_err(|err| {
            io::Error::new(io::ErrorKind::InvalidData, err)
        })?;

    return match key.as_ref() {
        "PRIORITY" => {
            let p = buf[length] as u64;
            Ok((key, p.into()))
        }
        "_SOURCE_MONOTONIC_TIMESTAMP" => {
            let mut ts = 0u64;
            for i in pos..length {
                let c = buf[i];
                if c.is_ascii_digit() {
                    ts = ts * 10 + (c - b'0') as u64
                } else {
                    break;
                }
            }

            Ok((key, ts.into()))
        }
        _ => {
            let value = String::from_utf8(buf[pos..length].to_vec()).map_err(|err| {
                io::Error::new(io::ErrorKind::InvalidData, err)
            })?;
            Ok((key, value.into()))
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
            let newline_pos = buf[0..read_to]
                .iter()
                .position(|b| *b == b'\n');

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
                    // TODO: implement it
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
mod tests {
    use super::*;

    use tokio::fs::File;
    use tokio::io::AsyncRead;
    use tokio_stream::StreamExt;
    use tokio_util::codec::{FramedRead, BytesCodec};

    #[tokio::test]
    async fn test_codec() {
        let reader = tokio::fs::File::open("testdata/journal_export.txt").await.unwrap();
        let mut stream = FramedRead::new(reader, EntryCodec::new());

        let mut count = 0;
        while let Some(result) = stream.next().await {
            match result {
                Ok(fields) => {
                    println!("fields: {:?}", fields);

                    count += 1;
                    if count == 3 {
                        return;
                    }
                }

                Err(err) => {
                    println!("err: {:?}", err)
                }
            }
        }
    }
}