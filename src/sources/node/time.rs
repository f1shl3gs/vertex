/// Exposes the current system time

use crate::{
    tags,
    gauge_metric,
    event::{Metric, MetricValue},
};
use std::{
    ffi::CStr,
};

pub async fn gather() -> Result<Vec<Metric>, ()> {
    let local_now = chrono::Local::now();
    let offset = local_now.offset()
        .local_minus_utc() as f64;

    let now_sec = local_now.timestamp_nanos() as f64 / 1e9;

    // TODO: we should and possiblily get TZ with chrono, cause
    // the offset is already got
    let tz = libc_timezone();

    Ok(vec![
        gauge_metric!(
            "node_time_seconds",
            "System time in seconds since epoch (1970)",
            now_sec
        ),
        gauge_metric!(
            "node_time_time_zone",
            "System time zone offset in seconds",
            offset,
            "time_zone" => tz.as_str()
        ),
    ])
}

fn libc_timezone() -> String {
    unsafe {
        let sec = 0i64;
        let sec = sec as libc::time_t;
        let mut out = std::mem::zeroed();

        if libc::localtime_r(&sec, &mut out).is_null() {
            panic!("syscall localtime_r failed, {}", std::io::Error::last_os_error());
        }

        let tz: &CStr = CStr::from_ptr(out.tm_zone) ;

        tz.to_str().unwrap().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local};
    use chrono_tz::{
        CET,
    };

    #[test]
    fn test_offset() {
        let now = Local::now();
        let cn = now.with_timezone(&CET);

        println!("{}", cn.format("%Z"));
    }

    use chrono_tz::{TZ_VARIANTS};
    use std::ffi::CStr;

    #[test]
    fn offset() {
        let local_now = chrono::Local::now();
        let offset = local_now.offset()
            .local_minus_utc();

        println!("offset: {}", offset);

        for tz in TZ_VARIANTS.iter() {
            println!("{:?}", tz);
        }
    }

    #[test]
    fn libc_localtime_r() {
        unsafe {
            let sec = 0i64;
            let sec = sec as libc::time_t;
            let mut out = std::mem::zeroed();

            if libc::localtime_r(&sec, &mut out).is_null() {
                panic!("xx")
            }

            println!("{:?}", out);

            let tz: &CStr = unsafe { CStr::from_ptr(out.tm_zone) };
            let tz = tz.to_str().unwrap();
            println!("{}", tz);
        }
    }
}