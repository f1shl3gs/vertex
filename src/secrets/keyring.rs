use std::collections::HashMap;

use configurable::configurable_component;
use framework::secret::{Error, SecretStore};

/// On Linux the kernel keyring in the user scope is used to store the secrets
#[configurable_component(secret, name = "keyring")]
struct Config {}

#[async_trait::async_trait]
#[typetag::serde(name = "keyring")]
impl SecretStore for Config {
    async fn retrieve(&self, keys: Vec<String>) -> Result<HashMap<String, String>, Error> {
        let mut data = HashMap::with_capacity(keys.len());

        for key in keys {
            let id = request_key("user", &key)?;
            let value = keyctl_read(id)?;

            data.insert(key, value);
        }

        Ok(data)
    }
}

fn request_key(typ: &str, key: &str) -> Result<i32, Error> {
    use std::ffi::CString;

    #[inline]
    fn to_cstring(input: &str) -> Result<CString, Error> {
        CString::new(input.as_bytes()).map_err(|_| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid string",
            ))
        })
    }

    let typ = to_cstring(typ)?;
    let desc = to_cstring(key)?;

    let ret = unsafe { libc::syscall(libc::SYS_request_key, typ.as_ptr(), desc.as_ptr(), 0, 0) };
    if ret == -1 {
        let errno = unsafe { *libc::__errno_location() as i32 };
        if errno == libc::ENOKEY {
            return Err(Error::NotFound(desc.to_string_lossy().to_string()));
        }

        return Err(std::io::Error::last_os_error().into());
    }

    Ok(ret as i32)
}

fn keyctl_read(id: i32) -> std::io::Result<String> {
    let mut data = Vec::with_capacity(4);

    loop {
        let ret = unsafe {
            libc::syscall(
                libc::SYS_keyctl,
                libc::KEYCTL_READ,
                id,
                data.as_mut_ptr() as *mut libc::c_void,
                data.capacity(),
            )
        };
        if ret < 0 {
            return Err(std::io::Error::last_os_error());
        }

        if ret as usize <= data.capacity() {
            unsafe { data.set_len(ret as usize) };
            break;
        }

        // grow
        data.reserve_exact(ret as usize);
    }

    String::from_utf8(data)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
