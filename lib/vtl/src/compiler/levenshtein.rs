pub fn distance(s1: &[u8], s2: &[u8]) -> usize {
    // Early exit if one of the strings is empty
    let v1len = s1.len();
    let v2len = s2.len();
    if v1len == 0 {
        return v2len;
    }
    if v2len == 0 {
        return v1len;
    }

    #[inline]
    fn min3<T: Ord>(v1: T, v2: T, v3: T) -> T {
        std::cmp::min(v1, std::cmp::min(v2, v3))
    }

    #[inline]
    fn delta(x: u8, y: u8) -> usize {
        if x == y {
            0
        } else {
            1
        }
    }

    let mut column: Vec<usize> = (0..v1len + 1).collect();
    for x in 1..v2len + 1 {
        column[0] = x;
        let mut lastdiag = x - 1;

        for y in 1..v1len + 1 {
            let olddiag = column[y];
            column[y] = min3(
                column[y] + 1,
                column[y - 1] + 1,
                lastdiag + delta(s1[y - 1], s2[x - 1]),
            );
            lastdiag = olddiag;
        }
    }

    column[v1len]
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
