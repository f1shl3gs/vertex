use std::cmp::Ordering;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BigUint(Vec<u64>);

impl BigUint {
    fn zero() -> Self {
        Self(Vec::new())
    }

    fn one() -> Self {
        Self(vec![1])
    }

    pub fn from_be_bytes(bytes: &[u8]) -> Self {
        let mut limbs = Vec::with_capacity(bytes.len().div_ceil(8));
        for chunk in bytes.rchunks(8) {
            let mut limb = 0u64;
            for byte in chunk {
                limb = (limb << 8) | (*byte as u64);
            }
            limbs.push(limb);
        }
        let mut out = Self(limbs);
        out.normalize();
        out
    }

    fn to_be_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.0.len() * 8);
        for limb in self.0.iter().rev() {
            out.extend_from_slice(&limb.to_be_bytes());
        }
        match out.iter().position(|b| *b != 0) {
            Some(i) => out[i..].to_vec(),
            None => vec![0],
        }
    }

    pub fn to_fixed_be_bytes(&self, len: usize) -> Vec<u8> {
        if self.0.is_empty() {
            return vec![0; len];
        }
        let bytes = self.to_be_bytes();
        let mut out = vec![0; len.saturating_sub(bytes.len())];
        out.extend_from_slice(&bytes);
        out
    }

    pub fn bit_len(&self) -> usize {
        match self.0.last() {
            Some(limb) => (self.0.len() - 1) * 64 + (64 - limb.leading_zeros() as usize),
            None => 0,
        }
    }

    pub fn bit(&self, bit: usize) -> bool {
        let limb = bit / 64;
        let shift = bit % 64;
        self.0
            .get(limb)
            .map(|limb| limb & (1u64 << shift) != 0)
            .unwrap_or(false)
    }

    /// `self^exponent mod modulus` using CIOS Montgomery multiplication and
    /// left-to-right binary square-and-multiply.
    ///
    /// Consumes `self` so the reduced-and-padded base buffer can be moved
    /// directly into the Montgomery loop without an extra copy.
    ///
    /// Returns an error if `modulus` is zero or even.
    pub fn mod_exp(self, exponent: &Self, modulus: &Self) -> Result<Self, super::Error> {
        if modulus.0.is_empty() || modulus.0[0] & 1 == 0 {
            return Err(super::Error::InvalidPublicKey(
                "modulus must be odd and non-zero",
            ));
        }

        let len = modulus.0.len();
        let n = &modulus.0[..];
        let n_inv_neg = inv_mod_2_64(n[0]).wrapping_neg();
        let r2 = r_squared_mod_n(modulus);

        // Reduce base mod modulus and pad to `len` limbs in one move-friendly chain.
        let mut base = self.reduce_into(modulus).0;
        base.resize(len, 0);

        // Pre-allocated working buffers — mont_mul_into is fully in-place.
        let mut scratch = vec![0u64; len + 2];
        let mut cur = vec![0u64; len];
        let mut nxt = vec![0u64; len];
        let mut base_mont = vec![0u64; len];

        // Move base into Montgomery domain: base_mont = base * R mod n.
        mont_mul_into(&mut base_mont, &base, &r2.0, n, n_inv_neg, &mut scratch);

        // 1 in the Montgomery domain is R mod n; we obtain it via mont_mul(1, R²).
        let mut one_padded = vec![0u64; len];
        one_padded[0] = 1;
        mont_mul_into(&mut cur, &one_padded, &r2.0, n, n_inv_neg, &mut scratch);

        for bit in (0..exponent.bit_len()).rev() {
            mont_mul_into(&mut nxt, &cur, &cur, n, n_inv_neg, &mut scratch);
            std::mem::swap(&mut cur, &mut nxt);
            if exponent.bit(bit) {
                mont_mul_into(&mut nxt, &cur, &base_mont, n, n_inv_neg, &mut scratch);
                std::mem::swap(&mut cur, &mut nxt);
            }
        }

        // Leave the Montgomery domain: result = cur * 1 * R⁻¹ mod n.
        mont_mul_into(&mut nxt, &cur, &one_padded, n, n_inv_neg, &mut scratch);

        let mut result = BigUint(nxt);
        result.normalize();
        Ok(result)
    }

    fn normalize(&mut self) {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    /// Bit-by-bit remainder, consumes `self`. Fast path returns `self` unchanged
    /// when it is already reduced (the common RSA-OAEP case where the encoded
    /// message starts with a `0x00` byte and is therefore < modulus).
    fn reduce_into(self, modulus: &Self) -> Self {
        if self.cmp(modulus) == Ordering::Less {
            return self;
        }
        let mut result = Self::zero();
        for bit in (0..self.bit_len()).rev() {
            shl1(&mut result.0);
            if self.bit(bit) {
                match result.0.first_mut() {
                    Some(limb) => *limb |= 1,
                    None => result.0.push(1),
                }
            }
            if result.cmp(modulus) != Ordering::Less {
                result = result.sub(modulus);
            }
        }
        result.normalize();
        result
    }

    fn sub(&self, rhs: &Self) -> Self {
        debug_assert!(self.cmp(rhs) != Ordering::Less);
        let mut out = Vec::with_capacity(self.0.len());
        let mut borrow: u128 = 0;
        for i in 0..self.0.len() {
            let left = self.0[i] as u128;
            let right = rhs.0.get(i).copied().unwrap_or(0) as u128 + borrow;
            if left >= right {
                out.push((left - right) as u64);
                borrow = 0;
            } else {
                out.push(((1u128 << 64) + left - right) as u64);
                borrow = 1;
            }
        }
        let mut out = Self(out);
        out.normalize();
        out
    }
}

impl Ord for BigUint {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.len().cmp(&other.0.len()) {
            Ordering::Equal => self.0.iter().rev().cmp(other.0.iter().rev()),
            ordering => ordering,
        }
    }
}

impl PartialOrd for BigUint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// `(t + a*b + c)` packed into `(low, high)`. Core MAC for schoolbook multiplication.
#[inline]
fn mac(t: u64, a: u64, b: u64, c: u64) -> (u64, u64) {
    let r = t as u128 + (a as u128) * (b as u128) + c as u128;
    (r as u64, (r >> 64) as u64)
}

#[inline]
fn shl1(limbs: &mut Vec<u64>) {
    let mut carry = 0u64;
    for limb in limbs.iter_mut() {
        let new = ((*limb as u128) << 1) | carry as u128;
        *limb = new as u64;
        carry = (new >> 64) as u64;
    }
    if carry != 0 {
        limbs.push(carry);
    }
}

/// In-place CIOS Montgomery multiplication.
///
/// Computes `out = a * b * R⁻¹ mod n` with `R = 2^(64*n.len())`.
/// `out`, `a`, `b`, `n` must all have exactly the same length;
/// `scratch` must have length `n.len() + 2` and is fully overwritten.
///
/// `out` may not alias `a`, `b`, or `n`. (`a` may alias `b` — used for squaring.)
fn mont_mul_into(
    out: &mut [u64],
    a: &[u64],
    b: &[u64],
    n: &[u64],
    n_inv_neg: u64,
    scratch: &mut [u64],
) {
    let len = n.len();
    debug_assert_eq!(out.len(), len);
    debug_assert_eq!(a.len(), len);
    debug_assert_eq!(b.len(), len);
    debug_assert_eq!(scratch.len(), len + 2);

    let t = scratch;
    t.fill(0);

    for &bi in b {
        // T += a * b[i]
        let mut carry = 0u64;
        for j in 0..len {
            let (lo, hi) = mac(t[j], a[j], bi, carry);
            t[j] = lo;
            carry = hi;
        }
        let (sum, c) = t[len].overflowing_add(carry);
        t[len] = sum;
        t[len + 1] = c as u64;

        // m = T[0] * (-n⁻¹) mod 2⁶⁴; then T = (T + m * n) / 2⁶⁴.
        // The low 64 bits of T[0] + m * n[0] are guaranteed zero by construction
        // and are discarded; the rest of T shifts down by one limb.
        let m = t[0].wrapping_mul(n_inv_neg);
        let (_, mut carry) = mac(t[0], m, n[0], 0);
        for j in 1..len {
            let (lo, hi) = mac(t[j], m, n[j], carry);
            t[j - 1] = lo;
            carry = hi;
        }
        let (sum, c) = t[len].overflowing_add(carry);
        t[len - 1] = sum;
        t[len] = t[len + 1] + (c as u64);
    }

    let top = t[len];
    out.copy_from_slice(&t[..len]);

    // Final conditional subtraction: result is in [0, 2N), reduce to [0, N).
    if top != 0 || cmp_padded(out, n) != Ordering::Less {
        let mut borrow: u128 = 0;
        for j in 0..len {
            let left = out[j] as u128;
            let right = n[j] as u128 + borrow;
            if left >= right {
                out[j] = (left - right) as u64;
                borrow = 0;
            } else {
                out[j] = ((1u128 << 64) + left - right) as u64;
                borrow = 1;
            }
        }
        debug_assert_eq!(borrow as u64, top);
    }
}

fn cmp_padded(a: &[u64], b: &[u64]) -> Ordering {
    debug_assert_eq!(a.len(), b.len());
    for i in (0..a.len()).rev() {
        match a[i].cmp(&b[i]) {
            Ordering::Equal => continue,
            ord => return ord,
        }
    }
    Ordering::Equal
}

/// `n⁻¹ mod 2⁶⁴` via Newton iteration; `n` must be odd.
fn inv_mod_2_64(n: u64) -> u64 {
    debug_assert!(n & 1 == 1);
    // Any odd n satisfies n*n ≡ 1 (mod 8), so x ≡ n⁻¹ (mod 8) initially.
    // Each iteration roughly doubles the precision; 6 covers 64 bits.
    let mut x = n;
    for _ in 0..6 {
        x = x.wrapping_mul(2u64.wrapping_sub(n.wrapping_mul(x)));
    }
    x
}

/// `R² mod n` where `R = 2^(64 * n.limbs)`, returned padded to `n.limbs` limbs.
///
/// For a normalized modulus (top bit of top limb set — always true for a
/// conforming RSA modulus) we take a fast path: compute `R mod n` directly as
/// the single subtraction `R - n` (since `floor(R/n) = 1` for normalized n),
/// then square it via `64 * len` bit-doublings. That halves the work compared
/// to doubling from `1` for `128 * len` iterations.
///
/// For non-normalized n (small test moduli, anything where `R/n` could be
/// large) we fall back to the naive bit-doubling loop, since the fast-path
/// "subtract until < n" step would otherwise iterate `floor(R/n)` times.
fn r_squared_mod_n(n: &BigUint) -> BigUint {
    let len = n.0.len();
    let normalized = n.0.last().is_some_and(|limb| limb.leading_zeros() == 0);

    if !normalized {
        let mut r = BigUint::one();
        for _ in 0..(128 * len) {
            shl1(&mut r.0);
            if r.cmp(n) != Ordering::Less {
                r = r.sub(n);
            }
        }
        r.0.resize(len, 0);
        return r;
    }

    // Fast path: r = R - n in (len+1)-limb arithmetic. The normalized invariant
    // guarantees `floor(R/n) == 1`, so r is already in `[0, n)` after the
    // single subtraction.
    let mut padded = vec![0u64; len + 1];
    padded[len] = 1;
    let mut borrow: u128 = 0;
    for (p, &nl) in padded.iter_mut().zip(n.0.iter()) {
        let left = *p as u128;
        let right = nl as u128 + borrow;
        if left >= right {
            *p = (left - right) as u64;
            borrow = 0;
        } else {
            *p = ((1u128 << 64) + left - right) as u64;
            borrow = 1;
        }
    }
    debug_assert_eq!(padded[len] - borrow as u64, 0);
    padded.truncate(len);
    let mut r = BigUint(padded);
    r.normalize();
    debug_assert!(r.cmp(n) == Ordering::Less);

    // Square: doubling another 64*len times turns R mod n into R² mod n.
    for _ in 0..(64 * len) {
        shl1(&mut r.0);
        if r.cmp(n) != Ordering::Less {
            r = r.sub(n);
        }
    }

    r.0.resize(len, 0);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modular_exponentiation_matches_small_values() {
        let base = BigUint::from_be_bytes(&[4]);
        let exp = BigUint::from_be_bytes(&[13]);
        let modulus = BigUint::from_be_bytes(&[0x01, 0xf1]);
        let result = base.mod_exp(&exp, &modulus).unwrap();
        assert_eq!(result.to_fixed_be_bytes(2), vec![0x01, 0xbd]);
    }

    #[test]
    fn modular_exponentiation_handles_base_larger_than_modulus() {
        // 100^7 mod 13 = (100 mod 13)^7 mod 13 = 9^7 mod 13.
        // 9^2=81≡3, 9^4≡9, 9^6≡27≡1, 9^7≡9.
        let base = BigUint::from_be_bytes(&[100]);
        let exp = BigUint::from_be_bytes(&[7]);
        let modulus = BigUint::from_be_bytes(&[13]);
        let result = base.mod_exp(&exp, &modulus).unwrap();
        assert_eq!(result.to_fixed_be_bytes(1), vec![9]);
    }

    #[test]
    fn modular_exponentiation_with_multi_limb_modulus() {
        // 2^256 mod (2^128 - 159) — verifies multi-limb Montgomery path.
        let mut modulus_bytes = vec![0u8; 16];
        modulus_bytes[0] = 0xff;
        for b in &mut modulus_bytes[1..15] {
            *b = 0xff;
        }
        modulus_bytes[15] = 0xff - 158; // 2^128 - 159
        let modulus = BigUint::from_be_bytes(&modulus_bytes);

        let base = BigUint::from_be_bytes(&[2]);
        let exp = BigUint::from_be_bytes(&[0x01, 0x00]); // exp = 256

        let result = base.mod_exp(&exp, &modulus).unwrap();
        // 2^256 mod (2^128 - 159) = (2^128 mod m)^2 mod m = 159^2 mod m = 25281.
        assert_eq!(result.to_fixed_be_bytes(2), vec![0x62, 0xc1]);
    }

    #[test]
    fn modular_exponentiation_rejects_even_modulus() {
        let base = BigUint::from_be_bytes(&[5]);
        let exp = BigUint::from_be_bytes(&[3]);
        let modulus = BigUint::from_be_bytes(&[8]);
        assert!(base.mod_exp(&exp, &modulus).is_err());
    }

    #[test]
    fn modular_exponentiation_rejects_zero_modulus() {
        let base = BigUint::from_be_bytes(&[5]);
        let exp = BigUint::from_be_bytes(&[3]);
        let modulus = BigUint::zero();
        assert!(base.mod_exp(&exp, &modulus).is_err());
    }

    #[test]
    fn converts_between_big_endian_bytes_and_limbs() {
        let value = BigUint::from_be_bytes(&[
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ]);
        assert_eq!(
            value.to_be_bytes(),
            vec![
                0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
                0x32, 0x10,
            ]
        );
    }

    #[test]
    fn zero_serialization() {
        let zero = BigUint::zero();
        assert_eq!(zero.to_be_bytes(), vec![0]);
        assert_eq!(zero.to_fixed_be_bytes(4), vec![0, 0, 0, 0]);
        assert_eq!(zero.to_fixed_be_bytes(0), Vec::<u8>::new());
    }

    #[test]
    fn inv_mod_2_64_round_trip() {
        for &n in &[1u64, 3, 5, 65537, 0xffff_ffff_ffff_fffd] {
            let inv = inv_mod_2_64(n);
            assert_eq!(n.wrapping_mul(inv), 1, "n={n}");
        }
    }
}
