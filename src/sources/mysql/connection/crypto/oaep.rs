use rand::TryRng;
use rand::rngs::SysRng;
use sha1::{Digest, Sha1};

use super::Error;

const SHA1_LEN: usize = 20;

pub fn encode(message: &[u8], k: usize) -> Result<Vec<u8>, Error> {
    let mut seed = [0u8; SHA1_LEN];
    SysRng.try_fill_bytes(&mut seed)?;
    encode_with_seed(message, k, seed)
}

// SHA1("") — used as `lHash` for an empty label.
const SHA1_EMPTY: [u8; SHA1_LEN] = [
    0xda, 0x39, 0xa3, 0xee, 0x5e, 0x6b, 0x4b, 0x0d, 0x32, 0x55, 0xbf, 0xef, 0x95, 0x60, 0x18, 0x90,
    0xaf, 0xd8, 0x07, 0x09,
];

fn encode_with_seed(message: &[u8], k: usize, mut seed: [u8; SHA1_LEN]) -> Result<Vec<u8>, Error> {
    if k < 2 * SHA1_LEN + 2 || message.len() > k - 2 * SHA1_LEN - 2 {
        return Err(Error::MessageTooLong);
    }

    let ps_len = k - message.len() - 2 * SHA1_LEN - 2;
    let mut block = Vec::with_capacity(k - SHA1_LEN - 1);
    block.extend_from_slice(&SHA1_EMPTY);
    block.resize(SHA1_LEN + ps_len, 0);
    block.push(1);
    block.extend_from_slice(message);

    let block_mask = mgf1(&seed, k - SHA1_LEN - 1);
    xor_in_place(&mut block, &block_mask);

    let seed_mask = mgf1(&block, SHA1_LEN);
    xor_in_place(&mut seed, &seed_mask);

    let mut out = Vec::with_capacity(k);
    out.push(0);
    out.extend_from_slice(&seed);
    out.extend_from_slice(&block);
    Ok(out)
}

fn mgf1(seed: &[u8], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len.div_ceil(SHA1_LEN) * SHA1_LEN);
    for counter in 0..len.div_ceil(SHA1_LEN) {
        let mut hasher = Sha1::new();
        hasher.update(seed);
        hasher.update((counter as u32).to_be_bytes());
        out.extend_from_slice(&hasher.finalize());
    }
    out.truncate(len);
    out
}

fn xor_in_place(left: &mut [u8], right: &[u8]) {
    debug_assert_eq!(left.len(), right.len());
    for (left, right) in left.iter_mut().zip(right) {
        *left ^= right;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inverse of `encode_with_seed`, used only for round-trip tests.
    /// Returns the recovered message and seed.
    fn decode(em: &[u8]) -> (Vec<u8>, [u8; SHA1_LEN]) {
        let k = em.len();
        assert!(k >= 2 * SHA1_LEN + 2);
        assert_eq!(em[0], 0);

        let masked_seed = &em[1..1 + SHA1_LEN];
        let masked_db = &em[1 + SHA1_LEN..];

        let seed_mask = mgf1(masked_db, SHA1_LEN);
        let mut seed = [0u8; SHA1_LEN];
        for i in 0..SHA1_LEN {
            seed[i] = masked_seed[i] ^ seed_mask[i];
        }

        let block_mask = mgf1(&seed, k - SHA1_LEN - 1);
        let mut block = vec![0u8; k - SHA1_LEN - 1];
        for i in 0..block.len() {
            block[i] = masked_db[i] ^ block_mask[i];
        }

        // block = lhash || PS (zeros) || 0x01 || message
        let label_hash = Sha1::digest([]);
        assert_eq!(&block[..SHA1_LEN], label_hash.as_slice());
        let mut i = SHA1_LEN;
        while i < block.len() && block[i] == 0 {
            i += 1;
        }
        assert!(i < block.len() && block[i] == 1);
        (block[i + 1..].to_vec(), seed)
    }

    #[test]
    fn round_trip_recovers_message_and_seed() {
        let seed = [0x42u8; SHA1_LEN];
        let message = b"caching_sha2_password";
        let k = 128;
        let em = encode_with_seed(message, k, seed).unwrap();
        assert_eq!(em.len(), k);
        assert_eq!(em[0], 0);
        let (recovered, recovered_seed) = decode(&em);
        assert_eq!(recovered, message);
        assert_eq!(recovered_seed, seed);
    }

    #[test]
    fn round_trip_handles_empty_message_and_max_size() {
        let seed = [0x00u8; SHA1_LEN];
        let k = 64;
        // empty message
        let em = encode_with_seed(b"", k, seed).unwrap();
        assert_eq!(decode(&em).0, b"");
        // maximum message length: k - 2*SHA1_LEN - 2 = 22
        let big = vec![0xab; k - 2 * SHA1_LEN - 2];
        let em = encode_with_seed(&big, k, seed).unwrap();
        assert_eq!(decode(&em).0, big);
    }

    #[test]
    fn rejects_message_too_long() {
        let seed = [0u8; SHA1_LEN];
        let k = 64;
        let too_long = vec![0u8; k - 2 * SHA1_LEN - 1];
        assert!(matches!(
            encode_with_seed(&too_long, k, seed),
            Err(Error::MessageTooLong)
        ));
    }

    #[test]
    fn mgf1_of_empty_seed_matches_known_sha1() {
        // mgf1([], 20) = SHA1([0,0,0,0]).
        let out = mgf1(&[], SHA1_LEN);
        let expected = Sha1::digest([0u8, 0, 0, 0]);
        assert_eq!(out, expected.as_slice());
    }
}
