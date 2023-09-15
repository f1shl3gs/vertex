use std::ffi::CStr;

use event::{tags, Metric};

use super::Error;

/// Exposes the current system time
pub async fn gather() -> Result<Vec<Metric>, Error> {
    let local_now = chrono::Local::now();
    let offset = local_now.offset().local_minus_utc() as f64;
    let now_sec = local_now.timestamp_nanos_opt().unwrap() as f64 / 1e9;

    // TODO: we should and possibly get TZ with chrono, cause
    // the offset is already got
    let tz = libc_timezone();

    Ok(vec![
        Metric::gauge(
            "node_time_seconds",
            "System time in seconds since epoch (1970)",
            now_sec,
        ),
        Metric::gauge_with_tags(
            "node_time_time_zone",
            "System time zone offset in seconds",
            offset,
            tags!(
                "time_zone" => tz,
            ),
        ),
    ])
}

fn libc_timezone() -> String {
    unsafe {
        // https://github.com/rust-lang/libc/issues/1848
        #[cfg_attr(target_env = "musl", allow(deprecated))]
        let sec = 0 as libc::time_t;
        let mut out = std::mem::zeroed();

        if libc::localtime_r(&sec, &mut out).is_null() {
            panic!(
                "syscall localtime_r failed, {}",
                std::io::Error::last_os_error()
            );
        }

        let tz: &CStr = CStr::from_ptr(out.tm_zone);

        tz.to_str().unwrap().to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;

    #[test]
    fn libc_localtime_r() {
        unsafe {
            let sec = 0i64;
            let sec = sec as libc::time_t;
            let mut out = std::mem::zeroed();

            if libc::localtime_r(&sec, &mut out).is_null() {
                panic!("xx")
            }

            let tz: &CStr = CStr::from_ptr(out.tm_zone);
            tz.to_str().unwrap();
        }
    }
}
