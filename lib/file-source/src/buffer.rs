use std::io::{self, BufRead};
use bytes::BytesMut;

/// Read up to `max_size` bytes from `reader`, splitting by `delim`
///
/// The function reads up to `max_size` bytes from `reader`, splitting the input
/// by `delim`. If a `delim` is not found in `reader` before `max_size` bytes
/// are read then the reader is polled until `delim` is found and the results
/// are discarded. Else, the result is written into `buf`.
///
/// The return is unusual. In the Err case this function has not written into
/// `buf` and the caller should not examine its contents. In the Ok case if the
/// inner value is None the caller should retry the call as the buffering read
/// hit the end of the buffer but did not find a `delim` yet, indicating that
/// we've sheered a write or that there were no bytes available in the `reader`
/// and the `reader` was very sure about it. If the inner value is Some the
/// interior `usize` is the number of bytes written into `buf`.
///
/// Tweak of
/// <https://github.com/rust-lang/rust/blob/bf843eb9c2d48a80a5992a5d60858e27269f9575/src/libstd/io/mod.rs#L1471>.
///
/// # Performance
///
/// Benchmarks indicate that this function processes in the high single-digit
/// GiB/s range for buffers of length 1KiB. For buffers any smaller than this
/// the overhead of setup dominates our benchmarks
pub fn read_until_with_max_size<R: BufRead + ?Sized>(
    reader: &mut R,
    position: &mut FilePosition,
    delim: &[u8],
    buf: &mut BytesMut,
    max_size: usize,
) -> io::Result<Option<usize>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU8;
    use test::test::TestResult;

    fn qc_inner(chunks: Vec<Vec<u8>>, delim: u8, max_size: NonZeroU8) -> TestResult {
        todo!()
    }
}