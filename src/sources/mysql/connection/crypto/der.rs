use base64::{Engine, engine::general_purpose::STANDARD};

use super::{BigUint, Error};

pub struct PublicKey {
    modulus: BigUint,
    exponent: BigUint,
}

impl PublicKey {
    pub fn from_pem(pem: &[u8]) -> Result<Self, Error> {
        let (der, ty) = pem_to_der(pem)?;
        let (modulus, exponent) = match ty {
            PubKeyFileType::Pkcs1 => parse_pkcs1(&der)?,
            PubKeyFileType::Pkcs8 => parse_pkcs8(&der)?,
        };

        Ok(Self {
            modulus: BigUint::from_be_bytes(&modulus),
            exponent: BigUint::from_be_bytes(&exponent),
        })
    }

    /// Byte length of the modulus as encoded in DER. Used to size the
    /// fixed-width ciphertext output.
    pub fn modulus_bytes(&self) -> usize {
        self.modulus.bit_len().div_ceil(8)
    }

    pub fn modulus(&self) -> &BigUint {
        &self.modulus
    }

    pub fn exponent(&self) -> &BigUint {
        &self.exponent
    }
}

enum PubKeyFileType {
    Pkcs1,
    Pkcs8,
}

fn pem_to_der(pem: &[u8]) -> Result<(Vec<u8>, PubKeyFileType), Error> {
    let pem = pem.trim_ascii();
    let mut lines = pem.split(|b| *b == b'\n' || *b == b'\r');
    let header = lines
        .next()
        .ok_or(Error::InvalidPublicKey("missing PEM header"))?
        .trim_ascii();

    let (ty, expected_footer) = if header == b"-----BEGIN RSA PUBLIC KEY-----" {
        (PubKeyFileType::Pkcs1, &b"-----END RSA PUBLIC KEY-----"[..])
    } else if header == b"-----BEGIN PUBLIC KEY-----" {
        (PubKeyFileType::Pkcs8, &b"-----END PUBLIC KEY-----"[..])
    } else {
        return Err(Error::InvalidPublicKey("unrecognized PEM header"));
    };

    let mut body = Vec::with_capacity(pem.len());
    let mut saw_footer = false;
    for line in lines {
        let line = line.trim_ascii();
        if line == expected_footer {
            saw_footer = true;
            break;
        }
        if line.starts_with(b"-----") {
            return Err(Error::InvalidPublicKey("unexpected PEM boundary"));
        }
        body.extend_from_slice(line);
    }
    if !saw_footer {
        return Err(Error::InvalidPublicKey("missing PEM footer"));
    }

    let der = STANDARD
        .decode(&body)
        .map_err(|_| Error::InvalidPublicKey("invalid base64 body"))?;
    Ok((der, ty))
}

fn parse_pkcs1(data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), Error> {
    let (seq, rest) = parse_sequence(data)?;
    if !rest.is_empty() {
        return Err(Error::InvalidPublicKey(
            "trailing bytes after PKCS#1 SEQUENCE",
        ));
    }
    let (modulus, seq) = parse_integer(seq)?;
    let (exponent, seq) = parse_integer(seq)?;
    if !seq.is_empty() {
        return Err(Error::InvalidPublicKey(
            "trailing bytes inside PKCS#1 SEQUENCE",
        ));
    }
    Ok((modulus, exponent))
}

fn parse_pkcs8(data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), Error> {
    let (seq, rest) = parse_sequence(data)?;
    if !rest.is_empty() {
        return Err(Error::InvalidPublicKey(
            "trailing bytes after PKCS#8 SEQUENCE",
        ));
    }
    let (_, seq) = parse_sequence(seq)?;
    let (unused_bits, public_key, seq) = parse_bit_string(seq)?;
    if !seq.is_empty() {
        return Err(Error::InvalidPublicKey("trailing bytes after BIT STRING"));
    }
    if unused_bits != 0 {
        return Err(Error::InvalidPublicKey("BIT STRING has unused bits"));
    }
    parse_pkcs1(public_key)
}

fn parse_sequence(data: &[u8]) -> Result<(&[u8], &[u8]), Error> {
    if data.first() != Some(&0x30) {
        return Err(Error::InvalidPublicKey("expected SEQUENCE"));
    }
    let (len, data) = parse_len(&data[1..])?;
    if data.len() < len {
        return Err(Error::InvalidPublicKey("SEQUENCE length out of bounds"));
    }
    Ok((&data[..len], &data[len..]))
}

fn parse_integer(data: &[u8]) -> Result<(Vec<u8>, &[u8]), Error> {
    if data.first() != Some(&0x02) {
        return Err(Error::InvalidPublicKey("expected INTEGER"));
    }
    let (len, data) = parse_len(&data[1..])?;
    if data.len() < len {
        return Err(Error::InvalidPublicKey("INTEGER length out of bounds"));
    }
    Ok((trim_be_integer(&data[..len]).to_vec(), &data[len..]))
}

fn parse_bit_string(data: &[u8]) -> Result<(u8, &[u8], &[u8]), Error> {
    if data.first() != Some(&0x03) {
        return Err(Error::InvalidPublicKey("expected BIT STRING"));
    }
    let (len, data) = parse_len(&data[1..])?;
    if len == 0 || data.len() < len {
        return Err(Error::InvalidPublicKey("BIT STRING length out of bounds"));
    }
    Ok((data[0], &data[1..len], &data[len..]))
}

fn parse_len(data: &[u8]) -> Result<(usize, &[u8]), Error> {
    let first = *data
        .first()
        .ok_or(Error::InvalidPublicKey("DER length truncated"))?;
    if first & 0x80 == 0 {
        Ok((first as usize, &data[1..]))
    } else {
        let bytes = (first & 0x7f) as usize;
        if bytes > size_of::<usize>() || data.len() < 1 + bytes {
            return Err(Error::InvalidPublicKey("DER length truncated"));
        }
        let mut len = 0usize;
        for b in &data[1..=bytes] {
            len = (len << 8) | (*b as usize);
        }
        Ok((len, &data[bytes + 1..]))
    }
}

fn trim_be_integer(mut data: &[u8]) -> &[u8] {
    while data.len() > 1 && data[0] == 0 {
        data = &data[1..];
    }
    data
}
