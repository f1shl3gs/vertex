mod reader;
mod writer;

use std::fmt::{Debug, Display};
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::{Sink, Stream};
use pin_project::pin_project;
use snafu::Snafu;
use event::{DecodeBytes, EncodeBytes};

use crate::usage::BufferUsageData;

#[derive(Debug, Snafu)]
pub enum DataDirError {
    #[snafu(display("The configured data_dir {:?} does not exist, please create it and make sure the vector process can write to it", data_dir))]
    NotFound { data_dir: PathBuf },
    #[snafu(display("The configured data_dir {:?} is not writable by the vector process, please ensure vector can write to that directory", data_dir))]
    NotWritable { data_dir: PathBuf },
    #[snafu(display("Unable to look up data_dir {:?}: {:?}", data_dir, source))]
    Metadata {
        data_dir: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Unable to open data_dir {:?}: {:?}", data_dir, source))]
    Open {
        data_dir: PathBuf,
        source: std::io::Error,
    },
}

#[pin_project]
#[derive(Clone)]
pub struct Writer<T>
where
    T: Send + Sync + Unpin + Clone + EncodeBytes<T> + DecodeBytes<T>,
    <T as EncodeBytes<T>>::Error: Debug,
    <T as DecodeBytes<T>>::Error: Debug,
{
    #[pin]
    inner: writer::Writer<T>,
}

impl<T> Sink<T> for Writer<T>
where
    T: Send + Sync + Unpin + Clone + EncodeBytes<T> + DecodeBytes<T>,
    <T as EncodeBytes<T>>::Error: Debug,
    <T as DecodeBytes<T>>::Error: Debug + Display,
{
    type Error = ();

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn start_send(self: Pin<&mut Self>, _item: T) -> Result<(), Self::Error> {
        todo!()
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }
}

pub fn open<'a, T>(
    dir: &Path,
    name: &str,
    _max_size: usize,
    _buffer_usage_data: Arc<BufferUsageData>,
) -> Result<
    (
        Writer<T>,
        Box<dyn Stream<Item = T> + 'a + Unpin + Send>,
        super::Acker,
    ),
    DataDirError,
>
where
    T: 'a + Send + Sync + Unpin + Clone + EncodeBytes<T> + DecodeBytes<T>,
    <T as EncodeBytes<T>>::Error: Debug,
    <T as DecodeBytes<T>>::Error: Debug + Display,
{
    let path = dir.join(name);

    // Check data dir
    std::fs::metadata(path)
        .map_err(|err| match err.kind() {
            io::ErrorKind::PermissionDenied => DataDirError::NotWritable {
                data_dir: dir.into(),
            },
            io::ErrorKind::NotFound => DataDirError::NotFound {
                data_dir: dir.into(),
            },
            _ => DataDirError::Metadata {
                data_dir: dir.into(),
                source: err,
            },
        })
        .and_then(|m| {
            if m.permissions().readonly() {
                Err(DataDirError::NotWritable {
                    data_dir: dir.into(),
                })
            } else {
                Ok(())
            }
        })?;

    todo!()
}

/*#[derive(Default)]
pub struct Buffer<T> {
    phantom: PhantomData<T>,
}

impl<T> Buffer<T>
    where
        T: Send + Sync + Unpin + EncodeBytes<T> + DecodeBytes<T>,
        <T as EncodeBytes<T>>::Error: Debug,
        <T as DecodeBytes<T>>::Error: Debug,
{
    pub fn build(
        path: &Path,
        max_size: usize,
        buffer_usage_data: Arc<BufferUsageData>,
    ) -> Result<(Writer<T>, Reader<T>, Acker), DataDirError> {
        todo!()
    }
}*/
