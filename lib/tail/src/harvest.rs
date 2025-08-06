use std::collections::HashMap;
use std::fmt::Debug;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tokio::sync::oneshot::Sender;
use tokio::task::JoinHandle;
use tracing::{debug, error, warn};

use super::notify::Registration;
use super::{Checkpointer, Conveyor, Fingerprint, Provider, ReadFrom};

const CONCURRENCY: usize = 2048;

const CHECKPOINTER_FLUSH_INTERVAL: Duration = Duration::from_secs(2);

// harvest running in a loop scanning files and
//
// shutdown: used to force shutdown the background tasks
pub async fn harvest<P, M, C, S>(
    mut provider: P,
    read_from: ReadFrom,
    checkpointer: Checkpointer,
    conveyor: C,
    mut shutdown: S,
) -> std::io::Result<()>
where
    P: Provider<Metadata = M>,
    C: Conveyor<Metadata = M> + 'static,
    S: Future<Output = ()> + Unpin + Send + 'static,
    M: Debug + Send + Sync + 'static,
{
    let (registration, event_stream) = Registration::new(CONCURRENCY)?;

    let mut running: HashMap<Fingerprint, (PathBuf, Sender<()>, JoinHandle<()>)> =
        HashMap::with_capacity(CONCURRENCY);
    let mut ticker = tokio::time::interval(CHECKPOINTER_FLUSH_INTERVAL);

    loop {
        let paths = tokio::select! {
            // `biased` will cause select to poll the futures in the order they appear
            // from top to bottom.
            //
            // so this select can works like a priority queue
            biased;

            result = event_stream.wait_and_wake() => match result {
                Ok(_) => continue,
                Err(err) => {
                    warn!(
                        message = "wait and wake failed",
                        ?err
                    );

                    break
                }
            },
            _ = ticker.tick() => {
                if let Err(err) = checkpointer.flush() {
                    error!(
                        message = "flush checkpoints failed",
                        ?err
                    );
                }

                continue;
            }
            _ = &mut shutdown => {
                debug!(
                    message = "shutdown signal received"
                );

                break
            },
            result = provider.scan() => match result {
                Ok(paths) => paths.into_iter().filter_map(|(path , metadata)| {
                    match path.metadata() {
                        Ok(stat) => {
                            let size = stat.size();
                            let fingerprint = Fingerprint::from(&stat);

                            // // if this file is too old, ignore it and don't remove fingerprint
                            // // because we might need it in the near future
                            // if let Some(older_than) = ignore_older_than {
                            //     let Ok(modified) = stat.modified() else {
                            //         return None
                            //     };
                            //
                            //     return if now - older_than > modified {
                            //         Some((path, fingerprint, size, metadata))
                            //     } else {
                            //         None
                            //     }
                            // }

                            Some((path, fingerprint, size, metadata))
                        },
                        Err(_err) => None
                    }
                }).collect::<Vec<_>>(),
                Err(err) => {
                    error!(
                        message = "Error while watching paths",
                        %err
                    );
                    continue
                },
            },
        };

        // stop tailing files which is not exist anymore
        // - file deleted
        // - no logs appended, something like `ignore_older_than` triggered
        for (fingerprint, (path, trigger, handle)) in
            running.extract_if(|key, (_path, _tx, _handle)| {
                !paths.iter().any(|(_, item, _, _)| key == item)
            })
        {
            // sent might fail, if the rx is not polled
            drop(trigger);

            checkpointer.delete(&fingerprint);

            if let Err(err) = handle.await {
                error!(
                    message = "await running task finished failed",
                    ?path,
                    ?fingerprint,
                    ?err,
                );
            }
        }

        // adding new files to running map and start tailing it
        for (path, fingerprint, size, metadata) in paths {
            // tail task is running already
            if running.contains_key(&fingerprint) {
                // maybe we should update the path
                continue;
            }

            // We can't open and tail a non-exist file
            if !path.exists() {
                debug!(message = "file is not exist anymore", ?path);
                continue;
            }

            if running.len() >= CONCURRENCY {
                error!(
                    message = "too many files to tail",
                    limit = CONCURRENCY,
                    ?path,
                );

                continue;
            }

            let offset = match checkpointer.get(fingerprint) {
                Some(offset) => {
                    // file truncated
                    if size < offset.load(Ordering::Acquire) {
                        offset.store(size, Ordering::Release);
                    }

                    offset
                }
                None => match read_from {
                    ReadFrom::Beginning => checkpointer.insert(fingerprint, 0),
                    ReadFrom::End => checkpointer.insert(fingerprint, size),
                },
            };

            let reader = match registration.watch(&path, offset.load(Ordering::Acquire)) {
                Ok(reader) => reader,
                Err(err) => {
                    error!(message = "create new file reader failed", ?err, ?path);

                    continue;
                }
            };

            debug!(
                message = "start tailing file",
                ?path,
                %fingerprint,
                offset = offset.load(Ordering::Acquire),
                size
            );

            // Add new file and start tail
            let (trigger, shutdown) = tokio::sync::oneshot::channel::<()>();
            let task = conveyor.run(reader, metadata, offset, shutdown);

            // Although, single thread reading should be fine, but the decoding might
            // take a lot of time, so spawn to other thread is necessary
            let handle = tokio::spawn(async move {
                if let Err(_err) = task.await {
                    error!(message = "Error while tailing file");
                }
            });
            running.insert(fingerprint, (path, trigger, handle));
        }
    }

    // we should stop all running task first, then wait them finished
    for (_fingerprint, (path, trigger, handle)) in running {
        // send might fail, if the task is not polled
        drop(trigger);

        debug!(message = "await conveyor task finished successfully", ?path);

        if let Err(err) = handle.await {
            error!(message = "await tail task handle failed", ?path, ?err);
        }
    }

    if let Err(err) = checkpointer.flush() {
        warn!(message = "Flush checkpoints failed", ?err);
    }

    Ok(())
}
