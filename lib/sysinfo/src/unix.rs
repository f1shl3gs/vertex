pub fn kernel() -> Option<String> {
    let mut raw = std::mem::MaybeUninit::<libc::utsname>::zeroed();

    if unsafe { libc::uname(raw.as_mut_ptr()) } == 0 {
        let info = unsafe { raw.assume_init() };

        let release = info
            .release
            .iter()
            .filter(|c| **c != 0)
            .map(|c| *c as u8 as char)
            .collect::<String>();

        Some(release)
    } else {
        None
    }
}

pub fn os() -> Option<String> {
    let content = match std::fs::read_to_string("/etc/os-release") {
        Ok(content) => content,
        Err(_) => match std::fs::read_to_string("/etc/lsb-release") {
            Ok(content) => content,
            Err(_) => return None,
        },
    };

    let mut name = String::new();
    let mut version = String::new();

    for line in content.lines() {
        if let Some(stripped) = line.strip_prefix("NAME=") {
            name = stripped.replace('"', "");
        }

        if let Some(stripped) = line.strip_prefix("VERSION=") {
            version = stripped.replace('"', "");
        }
    }

    Some(format!("Linux {} {}", name, version))
}

pub fn machine_id() -> std::io::Result<String> {
    std::fs::read_to_string("/etc/machine-id").map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_kernel_version() {
        kernel().unwrap();
    }
}
