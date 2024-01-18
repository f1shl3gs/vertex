pub fn distance(s1: &[u8], s2: &[u8]) -> usize {
    // Early exit if one of the strings is empty
    let s1len = s1.len();
    let s2len = s2.len();
    if s1len == 0 {
        return s2len;
    }
    if s2len == 0 {
        return s1len;
    }

    #[inline]
    fn min3<T: Ord>(s1: T, s2: T, s3: T) -> T {
        std::cmp::min(s1, std::cmp::min(s2, s3))
    }

    #[inline]
    fn delta(x: u8, y: u8) -> usize {
        if x == y {
            0
        } else {
            1
        }
    }

    let mut column: Vec<usize> = (0..s1len + 1).collect();
    for x in 1..s2len + 1 {
        column[0] = x;
        let mut lastdiag = x - 1;

        for y in 1..s1len + 1 {
            let olddiag = column[y];
            column[y] = min3(
                column[y] + 1,
                column[y - 1] + 1,
                lastdiag + delta(s1[y - 1], s2[x - 1]),
            );
            lastdiag = olddiag;
        }
    }

    column[s1len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ld() {
        assert_eq!(distance("cot".as_bytes(), "cat".as_bytes()), 1);
        assert_eq!(distance("ct".as_bytes(), "cot".as_bytes()), 1);
        assert_eq!(distance("cat".as_bytes(), "ct".as_bytes()), 1);
        assert_eq!(distance("cat".as_bytes(), "ct".as_bytes()), 1);
        assert_eq!(distance("cat".as_bytes(), "cta".as_bytes()), 2);
        assert_eq!(distance("cat".as_bytes(), "".as_bytes()), 3);
        assert_eq!(distance("".as_bytes(), "cat".as_bytes()), 3);
        assert_eq!(distance("".as_bytes(), "".as_bytes()), 0);
    }
}
