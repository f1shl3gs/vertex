use sha1::{Digest, Sha1};
use sha2::Sha256;

use super::Error;

#[derive(Debug)]
pub struct AuthConfig {
    pub username: String,
    pub password: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AuthPlugin {
    // `mysql_native_password` deprecated by mysql, but still used a lot
    Native,
    // `caching_sha2_password` this is recommended, since 8.0
    CachingSha2,
    Sha256,
    Clear,
}

impl AuthPlugin {
    pub fn name(&self) -> &str {
        match self {
            AuthPlugin::Native => "mysql_native_password",
            AuthPlugin::CachingSha2 => "caching_sha2_password",
            AuthPlugin::Sha256 => "sha256_password",
            AuthPlugin::Clear => "mysql_clear_password",
        }
    }

    pub fn scramble(&self, password: &str, nonce: (&[u8], &[u8])) -> Vec<u8> {
        match self {
            AuthPlugin::Native => scramble_sha1(password, nonce),
            AuthPlugin::CachingSha2 => {
                // https://mariadb.com/kb/en/caching_sha2_password-authentication-plugin/
                scramble_sha256(password, nonce)
            }
            AuthPlugin::Clear => {
                let mut data = password.as_bytes().to_vec();
                data.push(0); // null terminate
                data
            }
            // https://mariadb.com/kb/en/sha256_password-plugin/
            AuthPlugin::Sha256 => {
                unimplemented!("sha256_password");
            }
        }
    }
}

impl TryFrom<&[u8]> for AuthPlugin {
    type Error = Error;

    fn try_from(buf: &[u8]) -> Result<Self, Self::Error> {
        let end = buf.iter().position(|&b| b == b'\0').unwrap_or(buf.len());
        match &buf[..end] {
            b"mysql_native_password" => Ok(AuthPlugin::Native),
            b"caching_sha2_password" => Ok(AuthPlugin::CachingSha2),
            b"sha256_password" => Ok(AuthPlugin::Sha256),
            b"clear" => Ok(AuthPlugin::Clear),
            _ => Err(Error::UnsupportedAuthPlugin(
                String::from_utf8_lossy(&buf[..end]).to_string(),
            )),
        }
    }
}

fn scramble_sha1(password: &str, nonce: (&[u8], &[u8])) -> Vec<u8> {
    // SHA1( password ) ^ SHA1( seed + SHA1( SHA1( password ) ) )
    // https://mariadb.com/kb/en/connection/#mysql_native_password-plugin

    let mut hasher = Sha1::new();

    hasher.update(password.as_bytes());
    let mut pwh = hasher.finalize_reset();

    hasher.update(pwh);
    let pwhh = hasher.finalize_reset();

    hasher.update(nonce.0);
    hasher.update(nonce.1);
    hasher.update(pwhh);

    xor_eq(&mut pwh, hasher.finalize().as_ref());

    pwh.to_vec()
}

fn scramble_sha256(password: &str, nonce: (&[u8], &[u8])) -> Vec<u8> {
    // XOR(SHA256(password), SHA256(seed, SHA256(SHA256(password))))
    // https://mariadb.com/kb/en/caching_sha2_password-authentication-plugin/#sha-2-encrypted-password

    let mut hasher = Sha256::new();

    hasher.update(password.as_bytes());

    let mut pwh = hasher.finalize_reset();

    hasher.update(pwh);

    let pwhh = hasher.finalize_reset();

    hasher.update(pwhh);
    hasher.update(nonce.0);
    hasher.update(nonce.1);

    let pwshh = hasher.finalize();

    xor_eq(&mut pwh, &pwshh);

    pwh.to_vec()
}

// XOR(x, y)
// If len(y) < len(x), wrap around inside y
pub fn xor_eq(x: &mut [u8], y: &[u8]) {
    let y_len = y.len();

    for i in 0..x.len() {
        x[i] ^= y[i % y_len];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scramble_sha256_is_invertible_by_server() {
        let password = "my_pwd";
        let nonce_first = b"0123456789".as_ref();
        let nonce_second = [0xABu8; 10].as_ref();

        let mut scramble = scramble_sha256(password, (nonce_first, nonce_second));

        let stage1 = Sha256::digest(password.as_bytes());
        let stage2 = Sha256::digest(stage1);

        let mut h = Sha256::new();
        h.update(stage2);
        h.update(nonce_first);
        h.update(nonce_second);
        let xor_pad = h.finalize();

        xor_eq(&mut scramble, &xor_pad);
        assert_eq!(&scramble[..], &stage1[..]);
    }
}
