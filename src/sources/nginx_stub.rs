use std::convert::TryFrom;
use nom::{
    bytes::complete::{tag, take_while_m_n}
};
use nom::character::complete::u64;
use nom::combinator::all_consuming;
use nom::error::ErrorKind;
use nom::sequence::{preceded, terminated, tuple};
use snafu::Snafu;

#[derive(Debug, Snafu, PartialEq)]
enum ParseError {
    #[snafu(display("failed to parse nginx stub status, kind: {:?}", kind))]
    NginxStubStatusParseError { kind: ErrorKind }
}

#[derive(Debug, PartialEq, Eq)]
struct NginxStubStatus {
    active: u64,
    accepts: u64,
    handled: u64,
    requests: u64,
    reading: u64,
    writing: u64,
    waiting: u64,
}

impl<'a> TryFrom<&'a str> for NginxStubStatus {
    type Error = ParseError;

    // The `ngx_http_stub_status_module` response:
    // https://github.com/nginx/nginx/blob/master/src/http/modules/ngx_http_stub_status_module.c#L137-L145
    fn try_from(input: &'a str) -> Result<Self, Self::Error> {
        // `usize::MAX` eq `18446744073709551615` (20 char)
        match all_consuming(tuple((
            preceded(tag("Active connections: "), u64),
            preceded(tag(" \nserver accepts handled requests\n "), u64),
            preceded(tag(" "), u64),
            preceded(tag(" "), u64),
            preceded(tag(" \nReading: "), u64),
            preceded(tag(" Writing: "), u64),
            terminated(preceded(tag(" Waiting: "), u64), tag(" \n"))
        )))(input)
        {
            Ok((_, (active, accepts, handled, requests, reading, writing, waiting))) => {
                Ok(NginxStubStatus {
                    active,
                    accepts,
                    handled,
                    requests,
                    reading,
                    writing,
                    waiting,
                })
            }

            Err(err) => match err {
                nom::Err::Error(err) => {
                    Err(ParseError::NginxStubStatusParseError { kind: err.code })
                }

                nom::Err::Incomplete(_) | nom::Err::Failure(_) => unreachable!()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nginx_stub_status_try_from() {
        let data = "Active connections: 291 \n\
                    server accepts handled requests\n \
                    16630948 16630948 31070465 \n\
                    Reading: 6 Writing: 179 Waiting: 106 \n";

        assert_eq!(
            NginxStubStatus::try_from(input).expect("valid data"),
            NginxStubStatus {
                active: 291,
                accepts: 16630948,
                handled: 16630948,
                requests: 31070465,
                reading: 6,
                writing: 179,
                waiting: 106
            }
        )
    }
}