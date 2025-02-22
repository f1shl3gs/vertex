use std::env::temp_dir;
use std::ffi::CString;
use std::io::Write;
use std::num::ParseIntError;
use std::process::{Command, Stdio};
use std::time::Duration;

use bytes::Bytes;
use chrono::Utc;
use http::header::CONTENT_TYPE;
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::Incoming;
use libc::c_char;
use tikv_jemalloc_ctl::stats;

use super::{DEFAULT_PROFILE_SECONDS, Error};

#[derive(Debug, thiserror::Error)]
pub enum HeapProfileError {
    #[error("failed to active profiling, {0}")]
    ActivateProf(tikv_jemalloc_ctl::Error),
    #[error("failed to deactivate profiling, {0}")]
    DeactivateProf(tikv_jemalloc_ctl::Error),
    #[error("failed to dump jemalloc profile to {0}, {1}")]
    DumpProf(String, tikv_jemalloc_ctl::Error),

    #[error(transparent)]
    Other(tikv_jemalloc_ctl::Error),
}

impl From<tikv_jemalloc_ctl::Error> for Error {
    fn from(err: tikv_jemalloc_ctl::Error) -> Self {
        Error::Heap(HeapProfileError::Other(err))
    }
}

// C string should end with a '\0'.
const PROF_ACTIVE: &[u8] = b"prof.active\0";
const PROF_DUMP: &[u8] = b"prof.dump\0";

fn activate_prof() -> Result<(), HeapProfileError> {
    unsafe {
        tikv_jemalloc_ctl::raw::update(PROF_ACTIVE, true)
            .map_err(HeapProfileError::ActivateProf)?;
    }

    Ok(())
}

fn deactivate_prof() -> Result<(), HeapProfileError> {
    unsafe {
        tikv_jemalloc_ctl::raw::update(PROF_ACTIVE, false)
            .map_err(HeapProfileError::DeactivateProf)?;
    }

    Ok(())
}

/// Dump the profile to the path
fn dump_prof(path: &str) -> Result<(), HeapProfileError> {
    let mut buf = CString::new(path).unwrap().into_bytes_with_nul();
    let ptr = buf.as_mut_ptr() as *mut c_char;

    unsafe {
        tikv_jemalloc_ctl::raw::write(PROF_DUMP, ptr)
            .map_err(|err| HeapProfileError::DumpProf(path.to_string(), err))?;
    }

    Ok(())
}

pub async fn profile(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Error> {
    use crate::extensions::pprof::heap::*;

    let mut seconds = DEFAULT_PROFILE_SECONDS;
    let mut jeprof = false;

    if let Some(query) = req.uri().query() {
        for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
            match key.as_ref() {
                "seconds" => {
                    seconds = value
                        .parse()
                        .map_err(|err: ParseIntError| Error::ParseQuery {
                            key: "seconds",
                            err: err.to_string(),
                        })?;
                }
                "jeprof" => {
                    jeprof = value == "true";
                }
                _ => {}
            }
        }
    }

    activate_prof()?;

    tokio::time::sleep(Duration::from_secs(seconds)).await;

    deactivate_prof()?;

    let now = Utc::now();
    let temp_file = temp_dir().join(format!("pprof_{}.heap", now.format("%Y%m%d_%H%M%S")));

    dump_prof(temp_file.as_path().to_string_lossy().as_ref())?;

    let data = if jeprof {
        let binary = std::env::current_exe()?;

        let mut child = Command::new("perl")
            .args([
                "/dev/stdin",
                "--show_bytes",
                binary.to_string_lossy().as_ref(),
                temp_file.to_string_lossy().as_ref(),
                "--svg",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(include_bytes!("jeprof.in"))
            .unwrap();

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let resp = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(output.stderr)))
                .unwrap();

            return Ok(resp);
        }

        output.stdout
    } else {
        std::fs::read(temp_file)?
    };

    let content_type = if jeprof {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    };

    let resp = Response::builder()
        .status(StatusCode::OK)
        .header("X-Content-Type-Options", "nosniff")
        .header("Content-Length", data.len())
        .header(CONTENT_TYPE, content_type)
        .body(Full::new(Bytes::from(data)))
        .unwrap();

    Ok(resp)
}

pub async fn allocs(_req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Error> {
    let allocated = stats::allocated::read()?;
    let active = stats::active::read()?;
    let metadata = stats::metadata::read()?;
    let resident = stats::resident::read()?;
    let mapped = stats::mapped::read()?;
    let retained = stats::retained::read()?;

    let body = format!(
        "allocated: {}\nactive:    {}\nmetadata:  {}\nresident:  {}\nmapped:    {}\nretained:  {}\n",
        allocated, active, metadata, resident, mapped, retained
    );

    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "text/plain")
        .body(Full::<Bytes>::new(body.into()))
        .unwrap();

    Ok(resp)
}
