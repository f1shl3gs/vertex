mod biguint;
mod der;
mod oaep;

use biguint::BigUint;

#[derive(Debug)]
pub enum Error {
    MessageTooLong,
    Random(rand::rngs::SysError),
    InvalidPublicKey(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MessageTooLong => f.write_str("message too long for RSA-OAEP"),
            Self::Random(err) => write!(f, "OS random source failed: {err}"),
            Self::InvalidPublicKey(reason) => write!(f, "invalid RSA public key: {reason}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MessageTooLong | Self::InvalidPublicKey(_) => None,
            Self::Random(err) => Some(err),
        }
    }
}

impl From<rand::rngs::SysError> for Error {
    fn from(err: rand::rngs::SysError) -> Self {
        Self::Random(err)
    }
}

/// Minimum acceptable RSA modulus, in bytes (2048 bits).
const MIN_MODULUS_BYTES: usize = 256;

/// Helper function to encrypt mysql password using a public key loaded from a server.
///
/// It uses RSAES-OAEP with SHA-1, which is what MySQL uses for the full
/// authentication request of `caching_sha2_password`.
pub fn encrypt(password: &[u8], key: &[u8]) -> Result<Vec<u8>, Error> {
    let key = der::PublicKey::from_pem(key)?;

    if key.modulus_bytes() < MIN_MODULUS_BYTES {
        return Err(Error::InvalidPublicKey("modulus shorter than 2048 bits"));
    }
    if !key.modulus().bit(0) {
        return Err(Error::InvalidPublicKey("modulus must be odd"));
    }
    let exponent = key.exponent();
    // First check rejects 0 and any even value; the second then only has to
    // reject e == 1 (the unique odd value with bit_len < 2).
    if !exponent.bit(0) {
        return Err(Error::InvalidPublicKey("exponent must be odd and non-zero"));
    }
    if exponent.bit_len() < 2 {
        return Err(Error::InvalidPublicKey("exponent must be at least 3"));
    }

    let encoded = oaep::encode(password, key.modulus_bytes())?;
    let base = BigUint::from_be_bytes(&encoded);

    Ok(base
        .mod_exp(key.exponent(), key.modulus())?
        .to_fixed_be_bytes(key.modulus_bytes()))
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use base64::{Engine, engine::general_purpose::STANDARD};

    use super::*;

    const MOD_BYTES: usize = 256;

    #[test]
    fn encrypts_with_pkcs1_public_key() {
        let key = public_key_pem(PubKeyFileType::Pkcs1);

        let encrypted = encrypt(b"p", key.as_bytes()).unwrap();

        assert_eq!(encrypted.len(), MOD_BYTES);
    }

    #[test]
    fn encrypts_with_pkcs8_public_key() {
        let key = public_key_pem(PubKeyFileType::Pkcs8);

        let encrypted = encrypt(b"p", key.as_bytes()).unwrap();

        assert_eq!(encrypted.len(), MOD_BYTES);
    }

    #[test]
    fn rejects_short_modulus() {
        let mut modulus = vec![0xff; 128]; // 1024 bits, too short
        modulus[0] = 0x80;
        modulus[127] |= 1;
        let pkcs1 = der_sequence(&[der_integer(&modulus), der_integer(&[0x01, 0x00, 0x01])]);
        let encoded = STANDARD.encode(pkcs1);
        let pem =
            format!("-----BEGIN RSA PUBLIC KEY-----\n{encoded}\n-----END RSA PUBLIC KEY-----\n");
        assert_matches!(
            encrypt(b"p", pem.as_bytes()),
            Err(Error::InvalidPublicKey(_))
        );
    }

    #[test]
    fn rejects_even_modulus() {
        let mut modulus = vec![0xff; MOD_BYTES];
        modulus[0] = 0x80;
        modulus[MOD_BYTES - 1] = 0xfe; // even
        let pkcs1 = der_sequence(&[der_integer(&modulus), der_integer(&[0x01, 0x00, 0x01])]);
        let encoded = STANDARD.encode(pkcs1);
        let pem =
            format!("-----BEGIN RSA PUBLIC KEY-----\n{encoded}\n-----END RSA PUBLIC KEY-----\n");
        assert_matches!(
            encrypt(b"p", pem.as_bytes()),
            Err(Error::InvalidPublicKey(_))
        );
    }

    #[test]
    fn rejects_exponent_one() {
        let mut modulus = vec![0xff; MOD_BYTES];
        modulus[0] = 0x80;
        let pkcs1 = der_sequence(&[der_integer(&modulus), der_integer(&[0x01])]);
        let encoded = STANDARD.encode(pkcs1);
        let pem =
            format!("-----BEGIN RSA PUBLIC KEY-----\n{encoded}\n-----END RSA PUBLIC KEY-----\n");
        assert_matches!(
            encrypt(b"p", pem.as_bytes()),
            Err(Error::InvalidPublicKey(_))
        );
    }

    #[derive(Clone, Copy)]
    enum PubKeyFileType {
        Pkcs1,
        Pkcs8,
    }

    fn public_key_pem(ty: PubKeyFileType) -> String {
        let mut modulus = vec![0xff; MOD_BYTES];
        modulus[0] = 0x80;
        let pkcs1 = der_sequence(&[der_integer(&modulus), der_integer(&[0x01, 0x00, 0x01])]);

        let der = match ty {
            PubKeyFileType::Pkcs1 => pkcs1,
            PubKeyFileType::Pkcs8 => der_sequence(&[
                der_sequence(&[
                    der_oid(&[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01]),
                    vec![0x05, 0x00],
                ]),
                der_bit_string(&pkcs1),
            ]),
        };

        let label = match ty {
            PubKeyFileType::Pkcs1 => "RSA PUBLIC KEY",
            PubKeyFileType::Pkcs8 => "PUBLIC KEY",
        };
        let encoded = STANDARD.encode(der);
        format!("-----BEGIN {label}-----\n{encoded}\n-----END {label}-----\n")
    }

    fn der_sequence(parts: &[Vec<u8>]) -> Vec<u8> {
        let body = parts.concat();
        der_tag(0x30, &body)
    }

    fn der_integer(value: &[u8]) -> Vec<u8> {
        let mut body = if value[0] & 0x80 != 0 {
            let mut body = vec![0];
            body.extend_from_slice(value);
            body
        } else {
            value.to_vec()
        };
        while body.len() > 1 && body[0] == 0 && body[1] & 0x80 == 0 {
            body.remove(0);
        }
        der_tag(0x02, &body)
    }

    fn der_oid(value: &[u8]) -> Vec<u8> {
        der_tag(0x06, value)
    }

    fn der_bit_string(value: &[u8]) -> Vec<u8> {
        let mut body = vec![0];
        body.extend_from_slice(value);
        der_tag(0x03, &body)
    }

    fn der_tag(tag: u8, body: &[u8]) -> Vec<u8> {
        let mut out = vec![tag];
        out.extend_from_slice(&der_len(body.len()));
        out.extend_from_slice(body);
        out
    }

    fn der_len(len: usize) -> Vec<u8> {
        if len < 128 {
            vec![len as u8]
        } else {
            let mut bytes = Vec::new();
            let mut len = len;
            while len > 0 {
                bytes.push(len as u8);
                len >>= 8;
            }
            bytes.reverse();
            let mut out = vec![0x80 | bytes.len() as u8];
            out.extend(bytes);
            out
        }
    }
}
