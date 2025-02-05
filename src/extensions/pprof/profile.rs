use std::num::ParseIntError;
use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use http::header::CONTENT_LENGTH;
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::Incoming;
use pprof::protos::Message;

use super::{Error, DEFAULT_FREQUENCY, DEFAULT_PROFILE_SECONDS};

pub async fn handle(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Error> {
    let mut seconds = DEFAULT_PROFILE_SECONDS;
    let mut frequency = DEFAULT_FREQUENCY;
    let mut flamegraph = false;

    if let Some(query) = req.uri().query() {
        url::form_urlencoded::parse(query.as_bytes())
            .into_iter()
            .try_for_each::<_, Result<(), Error>>(|(key, value)| {
                match key.as_ref() {
                    "seconds" => {
                        seconds = value
                            .parse()
                            .map_err(|err: ParseIntError| Error::ParseQuery {
                                key: "seconds",
                                err: err.to_string(),
                            })?
                    }
                    "frequency" => {
                        frequency =
                            value
                                .parse()
                                .map_err(|err: ParseIntError| Error::ParseQuery {
                                    key: "frequency",
                                    err: err.to_string(),
                                })?
                    }
                    "flamegraph" => flamegraph = value == "true",
                    _ => {}
                }

                Ok(())
            })?;
    }

    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(frequency as i32)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .map_err(Error::BuildProfiler)?;

    tokio::time::sleep(Duration::from_secs(seconds)).await;

    match guard.report().build() {
        Ok(report) => {
            let data = if flamegraph {
                let mut buf = BytesMut::new().writer();
                report.flamegraph(&mut buf)?;

                buf.into_inner().freeze()
            } else {
                // Generating protobuf content which can be processed with
                // `go tool pprof -svg xxx.pb`, which will generate a svg
                // too.
                //
                // Pyroscope will consume this kind of data too.
                let mut buf = BytesMut::new();
                let profile = report.pprof()?;
                profile
                    .encode(&mut buf)
                    .map_err(|err| Error::Protobuf(err.to_string()))?;

                buf.freeze()
            };

            let content_type = if flamegraph {
                "image/svg+xml"
            } else {
                "application/protobuf"
            };

            let resp = Response::builder()
                .status(StatusCode::OK)
                .header("X-Content-Type-Options", "nosniff")
                .header("Content-Type", content_type)
                .header(CONTENT_LENGTH, data.len())
                .body(Full::from(data))
                .unwrap();

            Ok(resp)
        }

        Err(err) => {
            error!(message = "Build report failed", ?err);

            let resp = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::default())
                .expect("should build 500 response");

            Ok(resp)
        }
    }
}
