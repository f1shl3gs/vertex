use std::fmt::{Display, Formatter, Result};

pub struct PrettyFloat(pub f64);

#[derive(PartialEq, Eq, Debug)]
enum NumberClass {
    Big,
    Medium,
    Small,
    Zero,
    Special,
    Unprintable,
}

impl PrettyFloat {
    fn cls(&self) -> NumberClass {
        let mut x = self.0;
        if !x.is_finite() {
            return NumberClass::Special;
        }
        if x < 0.0 {
            x = -x;
        }
        if x == 0.0 {
            return NumberClass::Zero;
        }
        if x > 99999.0 {
            return NumberClass::Big;
        }
        if x < 0.001 {
            return NumberClass::Small;
        }

        NumberClass::Medium
    }
}

impl Display for PrettyFloat {
    fn fmt(&self, fmt: &mut Formatter) -> Result {
        let mut width_min = fmt.width().unwrap_or(3);
        let mut width_max = fmt.precision().unwrap_or(12);
        let x = self.0;

        if width_min == 0 {
            width_min = 1;
        }

        if width_max == 0 {
            return Ok(());
        }

        if width_min > width_max {
            width_max = width_min;
        }

        use NumberClass::*;
        let mut c = self.cls();

        if c == Special {
            let q = format!("{}", x);
            return if q.len() <= width_max {
                write!(fmt, "{:w$}", q, w = width_min)
            } else {
                write!(fmt, "{:.p$}", "########", p = width_max)
            };
        }
        if c == Zero {
            return if width_max < 3 || width_min < 3 {
                write!(fmt, "{:w$}", "0", w = width_min)
            } else {
                write!(fmt, "{:.p$}", 0.0, p = (width_min - 2))
            };
        }

        let probe_for_medium_mode;
        if c == Medium {
            probe_for_medium_mode = format!("{:.0}", x);
            let length_of_integer_part = probe_for_medium_mode.len();

            match length_of_integer_part {
                l if l > width_max => {
                    c = Big;
                }
                l if l + 1 >= width_max => {
                    if probe_for_medium_mode != "0" {
                        // print as integer
                        return write!(fmt, "{:w$.0}", x, w = width_min);
                    } else {
                        c = Unprintable;
                    }
                }
                _ => {
                    // Enouch room to try fractional part
                    // Check if it would be all zeroes
                    let probe = format!("{:.p$}", x, p = (width_max - 1 - length_of_integer_part),);

                    let mut num_zeroes = 0;
                    let mut num_digits = 0;
                    let mut significant_zeroes = false;

                    for c in probe.chars() {
                        match c {
                            '0' => {
                                num_digits += 1;
                                if !significant_zeroes {
                                    num_zeroes += 1;
                                }
                            }
                            '.' => {}
                            '-' => {}
                            _ => {
                                num_digits += 1;
                                significant_zeroes = true;
                            }
                        }
                    }

                    assert!(num_digits > 0);

                    if (num_zeroes * 100 / num_digits) > 80 {
                        // Too many zeroes, too few actual digits
                        c = Small;
                    }
                }
            }

            if c == Medium {
                // b fits max_width, but there may be opportunities to chip off zeroes
                let mut b = format!("{:.p$}", x, p = (width_max - 1 - length_of_integer_part));
                let first_digit_of_probe = probe_for_medium_mode.bytes().next();
                if first_digit_of_probe == Some(b'1') && first_digit_of_probe != b.bytes().next() {
                    let b2 = format!(
                        "{:.p$}",
                        x,
                        p = (width_max - 1 - length_of_integer_part + 1)
                    );
                    if b2.len() <= width_max {
                        b = b2;
                    }
                }
                let mut end = b.len();
                if b.contains('.') {
                    loop {
                        if end <= width_min {
                            break;
                        }
                        if end < 3 {
                            break;
                        }
                        if !b[0..end].ends_with('0') {
                            break;
                        }
                        if b[0..(end - 1)].ends_with('.') {
                            // protect one zero after '.'
                            break;
                        }

                        end -= 1;
                    }
                }
                let b = &b[0..end];
                for _ in b.len()..width_min {
                    write!(fmt, " ")?;
                }
                return write!(fmt, "{}", b);
            }
        }

        match c {
            Zero | Special | Medium => unreachable!(),
            Big | Small => {
                let probe = format!("{:.0e}", x);
                let mut minimum = probe.len();
                if minimum > width_max {
                    if c == Big {
                        c = Unprintable;
                    } else {
                        return write!(fmt, "{:w$}", 0.0, w = width_min);
                    }
                } else if minimum == width_max {
                    return write!(fmt, "{}", probe);
                } else if minimum == width_max - 1 {
                    // Can't increase precision because of we need to add a `.` as well
                    return write!(fmt, " {}", probe);
                } else {
                    let probe2 = format!("{:.p$e}", x, p = (width_max - minimum - 1));
                    if probe2.len() > width_max {
                        minimum += probe2.len() - width_max;
                    }
                    let mut zeroes_before_e = 0;
                    let mut zeroes_in_a_row = 0;
                    for c in probe2.chars() {
                        match c {
                            '0' => zeroes_in_a_row += 1,
                            'e' | 'E' => {
                                zeroes_before_e = zeroes_in_a_row;
                            }
                            _ => zeroes_in_a_row = 0,
                        }
                    }
                    let zeroes_to_chip_away = zeroes_before_e.min(width_max - width_min);
                    return write!(
                        fmt,
                        "{:.p$e}",
                        x,
                        p = (width_max - minimum - 1 - zeroes_to_chip_away)
                    );
                }
            }
            Unprintable => (),
        }
        let _ = c;

        write!(
            fmt,
            "{:.p$}",
            "##################################",
            p = width_min
        )
    }
}
