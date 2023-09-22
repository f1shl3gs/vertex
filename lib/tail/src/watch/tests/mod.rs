mod experiment;

use std::fs;
use std::io::Write;

use bytes::Bytes;
use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};

use crate::watch::Watcher;
use crate::ReadFrom;

#[derive(Clone, Debug)]
enum FileWatcherAction {
    WriteLine(String),
    RotateFile,
    DeleteFile,
    TruncateFile,
    Read,
    Pause(u32),
    Exit,
}

impl Arbitrary for FileWatcherAction {
    fn arbitrary(g: &mut Gen) -> Self {
        let i: usize = *g.choose(&(0..100).collect::<Vec<_>>()).unwrap();
        match i {
            // There weights are more or less arbitrary. 'Pause' maybe doesn't have a use but
            // we keep it in place to allow for variations in file-system flushes.
            0..=50 => {
                const GEN_ASCII_STR_CHARSET: &[u8] =
                    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
                let ln_sz = *g.choose(&(1..32).collect::<Vec<_>>()).unwrap();
                FileWatcherAction::WriteLine(
                    std::iter::repeat_with(|| *g.choose(GEN_ASCII_STR_CHARSET).unwrap())
                        .take(ln_sz)
                        .map(|v| -> char { v.into() })
                        .collect(),
                )
            }
            51..=69 => FileWatcherAction::Read,
            70..=75 => {
                let pause = *g.choose(&(1..3).collect::<Vec<_>>()).unwrap();
                FileWatcherAction::Pause(pause)
            }
            76..=85 => FileWatcherAction::RotateFile,
            86..=90 => FileWatcherAction::TruncateFile,
            91..=95 => FileWatcherAction::DeleteFile,
            _ => FileWatcherAction::Exit,
        }
    }
}

// WriteLine writes an arbitrary line of text -- plus newline -- RotateFile rotates the file
// as a log rotator might etc etc. Our interpreter functions take these instructions and apply
// them to the system under test(SUT), being a file_watcher pointed at a certain directory
// on-disk. In this way we can drive the behaviour of file_watcher. Validation requires a
// model, which we scattered between the interpreters -- as the model varies slightly in the
// presence of truncation vs. not -- and FWFile.
pub struct FileWatcherFile {
    contents: Vec<u8>,
    read_index: usize,
    previous_read_size: usize,
    reads_available: usize,
}

// FileWatcherFile mimics an actual Unix file, at least for our purposes here. The operations
// available on FileWatcherFile have to do with reading and writing lines, truncation and resets,
// which mimic a delete/create cycle on the file system. The function `FileWatcherFile::read_line`
// is the most complex and you're warmly encouraged to read the documentation present there.
impl FileWatcherFile {
    pub fn new() -> Self {
        FileWatcherFile {
            contents: vec![],
            read_index: 0,
            previous_read_size: 0,
            reads_available: 0,
        }
    }

    pub fn reset(&mut self) {
        self.contents.truncate(0);
        self.read_index = 0;
        self.previous_read_size = 0;
        self.reads_available = 0;
    }

    pub fn truncate(&mut self) {
        self.reads_available = 0;
        self.contents.truncate(0);
    }

    pub fn write_line(&mut self, input: &str) {
        self.contents.extend_from_slice(input.as_bytes());
        self.contents.push(b'\n');
        self.reads_available += 1;
    }

    pub fn read_line(&mut self) -> Option<String> {
        // FWFile mimics a unix file being read in a buffered fashion,
        // driven by file_watcher. We _have_ to keep on top of where the
        // reader's read index -- called read_idx -- is between reads and
        // the size of the file -- called previous_read_size -- in the event
        // of truncation.
        //
        // If we detect in file_watcher that a truncation has happened then
        // the buffered reader is seeked back to 0. This is performed in
        // like kind when we reset read_idx to 0, as in the following case
        // where there are no reads available.
        if self.contents.is_empty() && self.reads_available == 0 {
            self.read_index = 0;
            self.previous_read_size = 0;
            return None;
        }
        // Now, the above is done only when nothing has been written to the
        // FWFile or the contents have been totally removed. The trickier
        // case is where there are maybe _some_ things to be read but the
        // read_idx might be mis-set owing to truncations.
        //
        // `read_line` is performed in a line-wise fashion. start_idx
        // and end_idx are pulled apart from one another to find the
        // start and end of the line, if there's a line to be found.
        let mut end_idx;
        let start_idx;
        // Here's where we do truncation detection. When our file has
        // shrunk, restart the search at zero index. If the file is the
        // same size -- implying that it's either not changed or was
        // truncated and then filled back in before a read could occur
        // -- we return None. Else, start searching at the present
        // read_idx.
        let max = self.contents.len();
        if self.previous_read_size > max {
            self.read_index = 0;
            start_idx = 0;
            end_idx = 0;
        } else if self.read_index == max {
            return None;
        } else {
            start_idx = self.read_index;
            end_idx = self.read_index;
        }
        // Seek end_idx forward until we hit the newline character.
        while self.contents[end_idx] != b'\n' {
            end_idx += 1;
            if end_idx == max {
                return None;
            }
        }
        // Produce the read string -- minus its newline character -- and
        // set the control variables appropriately.
        let ret = std::str::from_utf8(&self.contents[start_idx..end_idx]).unwrap();
        self.read_index = end_idx + 1;
        self.reads_available -= 1;
        self.previous_read_size = max;
        // There's a trick here. What happens if we _only_ read a
        // newline character. Well, that'll happen when truncations
        // cause trimmed reads and the only remaining character in the
        // line is the newline. Womp womp
        if !ret.is_empty() {
            Some(ret.to_string())
        } else {
            None
        }
    }
}

fn no_truncations(action: Vec<FileWatcherAction>) {
    let dir = tempfile::TempDir::new().expect("could not create tempdir");
    let path = dir.path().join("a_file.log");
    let mut fp = fs::File::create(&path).expect("could not create");
    let mut rotation_count = 0;
    let mut fw = Watcher::new(
        path.clone(),
        ReadFrom::Beginning,
        None,
        100_000,
        Bytes::from("\n"),
    )
    .expect("must be able to create file watcher");

    let mut files: Vec<FileWatcherFile> = vec![FileWatcherFile::new()];
    let mut read_index = 0;
    for action in action.iter() {
        match *action {
            FileWatcherAction::DeleteFile => {
                let _ = fs::remove_file(&path);
                assert!(!path.exists());
                files[0].reset();
                break;
            }
            FileWatcherAction::TruncateFile => {}
            FileWatcherAction::Pause(ps) => delay(ps),
            FileWatcherAction::Exit => break,
            FileWatcherAction::WriteLine(ref s) => {
                files[0].write_line(s);
                assert!(fp.write_all(s.as_bytes()).is_ok());
                assert!(fp.write_all(b"\n").is_ok());
                assert!(fp.flush().is_ok());
            }
            FileWatcherAction::RotateFile => {
                let mut new_path = path.clone();
                new_path.set_extension(format!("log.{}", rotation_count));
                rotation_count += 1;
                fs::rename(&path, &new_path).expect("could not rename");
                fp = fs::File::create(&path).expect("could not create");
                files.insert(0, FileWatcherFile::new());
                read_index += 1;
            }
            FileWatcherAction::Read => {
                let mut attempts = 10;
                while attempts > 0 {
                    match fw.read_line() {
                        Err(_) => {
                            unreachable!()
                        }
                        Ok(Some(line)) if line.is_empty() => {
                            attempts -= 1;
                            assert!(files[read_index].read_line().is_none());
                            continue;
                        }
                        Ok(None) => {
                            attempts -= 1;
                            assert!(files[read_index].read_line().is_none());
                            continue;
                        }
                        Ok(Some(line)) => {
                            let exp = files[read_index].read_line().expect("could not readline");
                            assert_eq!(exp.into_bytes(), line);
                            break;
                        }
                    }
                }
            }
        }
    }
}

#[inline]
fn delay(attempts: u32) {
    let delay = match attempts {
        0 => return,
        1 => 1,
        2 => 4,
        3 => 8,
        4 => 16,
        5 => 32,
        6 => 64,
        7 => 128,
        8 => 256,
        _ => 512,
    };

    let time = std::time::Duration::from_millis(delay);
    std::thread::sleep(time);
}

#[test]
fn file_watcher_no_truncation() {
    fn inner(actions: Vec<FileWatcherAction>) -> TestResult {
        no_truncations(actions);
        TestResult::passed()
    }

    QuickCheck::new()
        .tests(10000)
        .max_tests(100000)
        .quickcheck(inner as fn(Vec<FileWatcherAction>) -> TestResult);
}
