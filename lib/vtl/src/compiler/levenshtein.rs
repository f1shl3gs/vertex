pub fn distance<T1: AsRef<[u8]>, T2: AsRef<[u8]>>(s1: T1, s2: T2) -> usize {
    let v1 = s1.as_ref();
    let v2 = s2.as_ref();

    // Early exit if one of the strings is empty
    let v1len = v1.len();
    let v2len = v2.len();
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
                lastdiag + delta(v1[y - 1], v2[x - 1]),
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
        assert_eq!(distance("cot", "cat"), 1);
        assert_eq!(distance("ct", "cot"), 1);
        assert_eq!(distance("cat", "ct"), 1);
        assert_eq!(distance("cat", "ct"), 1);
        assert_eq!(distance("cat", "cta"), 2);
        assert_eq!(distance("cat", ""), 3);
        assert_eq!(distance("", "cat"), 3);
        assert_eq!(distance("", ""), 0);
    }
}
