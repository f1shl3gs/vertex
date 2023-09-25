use event::{tags, Metric};

use super::Error;

pub async fn gather() -> Result<Vec<Metric>, Error> {
    let mut u = libc::utsname {
        sysname: [0; 65],
        nodename: [0; 65],
        release: [0; 65],
        version: [0; 65],
        machine: [0; 65],
        domainname: [0; 65],
    };

    let v = unsafe { libc::uname(&mut u) };
    if v != 0 {
        warn!("call libc::uname failed, code {}", v as i8);
        return Err(Error::last_os_error());
    }

    let sysname = &to_string(u.sysname);
    let release = &to_string(u.release);
    let version = &to_string(u.version);
    let machine = &to_string(u.machine);
    let nodename = &to_string(u.nodename);
    let domainname = &to_string(u.domainname);

    Ok(vec![Metric::gauge_with_tags(
        "node_uname_info",
        "Labeled system information as provided by the uname system call.",
        1f64,
        tags!(
            "sysname" => sysname,
            "release" => release,
            "version" => version,
            "machine" => machine,
            "nodename" => nodename,
            "domainname" => domainname,
        ),
    )])
}

fn to_string(cs: [libc::c_char; 65]) -> String {
    let mut s = String::with_capacity(64);
    for c in cs {
        if c == 0 {
            break;
        }

        s.push(c as u8 as char);
    }

    s
}
