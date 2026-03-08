use std::borrow::Cow;
use std::ffi::OsStr;

use event::{Metric, tags};

use super::Error;

pub async fn collect() -> Result<Vec<Metric>, Error> {
    let mut uname = libc::utsname {
        sysname: [0; 65],
        nodename: [0; 65],
        release: [0; 65],
        version: [0; 65],
        machine: [0; 65],
        domainname: [0; 65],
    };

    let ret = unsafe { libc::uname(&mut uname) };
    if ret != 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    let sysname = to_string(&uname.sysname);
    let release = to_string(&uname.release);
    let version = to_string(&uname.version);
    let machine = to_string(&uname.machine);
    let nodename = to_string(&uname.nodename);
    let domainname = to_string(&uname.domainname);

    Ok(vec![Metric::gauge_with_tags(
        "node_uname_info",
        "Labeled system information as provided by the uname system call.",
        1f64,
        tags!(
            "domainname" => domainname,
            "machine" => machine,
            "nodename" => nodename,
            "sysname" => sysname,
            "release" => release,
            "version" => version,
        ),
    )])
}

fn to_string(buf: &[libc::c_char; 65]) -> Cow<'_, str> {
    use std::os::unix::ffi::OsStrExt;

    let length = buf.iter().position(|&byte| byte == 0).unwrap_or(buf.len());
    let bytes = unsafe { std::slice::from_raw_parts(buf.as_ptr().cast(), length) };

    OsStr::from_bytes(bytes).to_string_lossy()
}
