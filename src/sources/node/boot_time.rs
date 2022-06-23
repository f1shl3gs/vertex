use event::Metric;
use libc::{c_int, sysctl, timeval};

use super::Error;

pub fn gather() -> Result<Vec<Metric>, Error> {
    let ts = boot_time()?;

    // This conversion maintains the usec precision.
    let v = ts.tv_sec as f64 + (ts.tv_usec as f64 / (1000.0 * 1000.0));

    Ok(vec![Metric::gauge(
        "node_boot_time_seconds",
        "Unix time of last boot, including microseconds",
        v,
    )])
}

fn boot_time() -> Result<timeval, std::io::Error> {
    let mut tv = timeval {
        tv_sec: 0,
        tv_usec: 0,
    };

    let mut len = std::mem::size_of::<timeval>();
    let mut mib: [c_int; 2] = [libc::CTL_KERN, libc::KERN_BOOTTIME];

    unsafe {
        if sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            &mut tv as *mut timeval as *mut _,
            &mut len,
            std::ptr::null_mut(),
            0,
        ) < 0
        {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(tv)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_time() {
        let ts = boot_time().unwrap();
        println!("{}", ts.tv_sec)
    }
}
