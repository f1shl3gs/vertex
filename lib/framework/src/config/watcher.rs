use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::Error;

/// Per notify own documentation, it's advised to have delay of more than 30 sec,
/// so to avoid receiving repetitions of previous events on MacOS.
///
/// But, config and topology reload logic can handle:
///  - Invalid config, caused either by user or by data race.
///  - Frequent changes, caused by user/editor modifying/saving file in small chunk.
/// so we can use smaller, more responsive delay.
const DEFAULT_WATCH_DELAY: Duration = Duration::from_secs(1);

const RETRY_TIMEOUT: Duration = Duration::from_secs(10);

pub fn spawn_thread<'a>(
    config_paths: impl IntoIterator<Item = &'a PathBuf> + 'a,
    delay: impl Into<Option<Duration>>,
) -> Result<(), Error> {
    let delay = delay.into().unwrap_or(DEFAULT_WATCH_DELAY);
    let config_paths: Vec<_> = config_paths.into_iter().cloned().collect();

    // Create watcher now so not to miss any changes happening between
    // returning from this function and the thread starting.
    let mut watcher = Some(create_watcher(&config_paths)?);

    info!(message = "Watching configuration files");

    thread::spawn(move || loop {
        if let Some((mut watcher, receiver)) = watcher.take() {
            while let Ok(event) = receiver.recv() {
                if matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_)
                ) {
                    debug!(message = "Configuration file change detected", ?event);

                    // Consume events until delay amount of time has passed since the latest event.
                    while receiver.recv_timeout(delay).is_ok() {}

                    // We need to read paths to resolve any inode changes that may have happened.
                    // And we need to do it before raising sighup to avoid missing any change.
                    if let Err(err) = add_paths(&mut watcher, &config_paths) {
                        error!(message = "Failed to add files to watch", ?err);
                        break;
                    }

                    info!(message = "Configuration file changed.");
                    raise_sighup();
                } else {
                    debug!(message = "Ignoring event", ?event);
                }
            }
        }

        thread::sleep(RETRY_TIMEOUT);

        watcher = create_watcher(&config_paths)
            .map_err(|err| {
                error!(message = "Failed to create file watcher", ?err);
            })
            .ok();

        if watcher.is_some() {
            // Config files could have changed while we weren't watching,
            // so for a good measure raise SIGHUP and let reload logic
            // determine if anything changed.
            info!(message = "Speculating that configuration files have changed",);

            raise_sighup();
        }
    });

    Ok(())
}

fn raise_sighup() {
    use nix::sys::signal;

    let _ = signal::raise(signal::Signal::SIGHUP).map_err(|err| {
        error!(
            message = "Unable to reload configuration file. Restart Vertex to reload it",
            cause = %err
        )
    });
}

fn create_watcher(
    config_paths: &[PathBuf],
) -> Result<(RecommendedWatcher, Receiver<Event>), Error> {
    info!(message = "Creating configuration file watcher");

    let (sender, receiver) = channel();
    let mut watcher = RecommendedWatcher::new(
        move |result| match result {
            Ok(event) => {
                if let Err(err) = sender.send(event) {
                    warn!(message = "send notify event failed", ?err);
                }
            }

            Err(err) => {
                error!(message = "receive notify event failed", ?err);
            }
        },
        Config::default(),
    )?;

    add_paths(&mut watcher, config_paths)?;

    Ok((watcher, receiver))
}

fn add_paths(watcher: &mut RecommendedWatcher, config_paths: &[PathBuf]) -> Result<(), Error> {
    config_paths.iter().try_for_each(|path| {
        watcher
            .watch(path, RecursiveMode::NonRecursive)
            .map_err(Into::into)
    })
}

#[cfg(all(test, unix, not(target_os = "macos")))]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use testify::temp::{temp_file, temp_dir};
    use tokio::signal::unix::{signal, SignalKind};

    use super::*;

    async fn test(file: &mut File, timeout: Duration) -> bool {
        let mut signal = signal(SignalKind::hangup()).expect("Signal handlers should not panic");

        file.write_all(&[0]).unwrap();
        file.sync_all().unwrap();

        tokio::time::timeout(timeout, signal.recv()).await.is_ok()
    }

    #[tokio::test]
    async fn file_directory_update() {
        let delay = Duration::from_secs(3);
        let directory = temp_dir();
        let filepath = directory.join("test.txt");
        let mut file = File::create(&filepath).unwrap();

        spawn_thread(&[directory], delay).unwrap();

        assert!(test(&mut file, delay * 5).await)
    }

    #[tokio::test]
    async fn file_update() {
        let delay = Duration::from_secs(3);
        let filepath = temp_file();
        let mut file = File::create(&filepath).unwrap();

        spawn_thread(&[filepath], delay).unwrap();

        assert!(test(&mut file, delay * 5).await)
    }

    #[tokio::test]
    async fn sym_file_update() {
        let delay = Duration::from_secs(3);
        let filepath = temp_file();
        let sym_file = temp_file();
        let mut file = File::create(&filepath).unwrap();
        std::os::unix::fs::symlink(&filepath, &sym_file).unwrap();

        spawn_thread(&[sym_file], delay).unwrap();

        assert!(test(&mut file, delay * 5).await);
    }
}
