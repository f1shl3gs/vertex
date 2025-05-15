use std::io::{Error, Result};

#[cfg(unix)]
pub fn get() -> Result<String> {
    // SAFETY: There don't appear to be any safety requirements for calling
    // sysconf.
    let limit = unsafe { libc::sysconf(libc::_SC_HOST_NAME_MAX) };
    if limit == -1 {
        // It is in theory possible for sysconf to return -1 for a limit
        // but *not* set errno, in which case, io::Error::last_os_error
        // is indeterminate. But untangling that is super annoying
        // because std doesn't expose any unix-specific APIs for inspecting
        // the errno. (We could do it ourselves, but it just doesn't seem
        // worth doing?)
        return Err(Error::last_os_error());
    }

    let Ok(max_len) = usize::try_from(limit) else {
        let msg = format!("hostname max limit ({limit}) overflowed usize");
        return Err(Error::other(msg));
    };

    // max_len here includes the NUL terminator.
    let mut buf = vec![0; max_len];

    // SAFETY: The pointer we give is valid as it is derived directly from
    // a Vec. Similarly, `max_len` is the length of our Vec, and is thus
    // valid to write to.
    let rc = unsafe { libc::gethostname(buf.as_mut_ptr().cast::<libc::c_char>(), max_len) };
    if rc == -1 {
        return Err(Error::last_os_error());
    }

    // POSIX says that if the hostname is bigger than `max_len`, then it may
    // write a truncate name back that is not necessarily NUL terminated.
    // So if we can't find a NUL terminator, then just give up.
    let Some(zero_pos) = buf.iter().position(|&b| b == 0) else {
        return Err(Error::other("could not find NUL terminator in hostname"));
    };

    buf.truncate(zero_pos);

    Ok(unsafe { String::from_utf8_unchecked(buf) })
}
