use std::fs;
use std::io::Write;
use bytes::Bytes;
use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
use crate::ReadFrom;

use crate::watch::Watcher;

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

    pub fn read_line(&mut self) -> Option<String> {
        todo!()
    }
}

fn no_truncations(action: Vec<FileWatcherAction>) {
    let dir = tempfile::TempDir::new()
        .expect("could not create tempdir");
    let path = dir.path().join("a_file.log");
    let mut fp = fs::File::create(&path)
        .expect("could not create");
    let mut rotation_count = 0;
    let mut fw = Watcher::new(
        path.clone(),
        ReadFrom::Beginning,
        None,
        100_000,
        Bytes::from("\n"),
    ).expect("must be able to create file watcher");

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
                assert!(fp.write(s.as_bytes()).is_ok());
                assert!(fp.write(b"\n").is_ok());
                assert!(fp.flush().is_ok());
            }
            FileWatcherAction::RotateFile => {
                let mut new_path = path.clone();
                new_path.set_extension(format!("log.{}", rotation_count));
                rotation_count += 1;
                fs::rename(&path, &new_path)
                    .expect("could not rename");
                fp = fs::File::create(&path)
                    .expect("could not create");
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
                            let exp = files[read_index].read_line()
                                .expect("could not readline");
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
        _ => 512
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