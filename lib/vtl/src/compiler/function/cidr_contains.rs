use std::fmt::{Display, Formatter};
use std::net::IpAddr;
use std::str::FromStr;

use value::{Kind, Value};

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;
use crate::SyntaxError;

#[derive(Debug)]
enum Error {
    Addr,
    Bits,
    NoSeparator,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Addr => f.write_str("invalid addr"),
            Error::Bits => f.write_str("invalid bits"),
            Error::NoSeparator => f.write_str("no separator"),
        }
    }
}

/// Simple implement of CIDR, cause all we need is contains
///
/// https://en.wikipedia.org/wiki/Classless_Inter-Domain_Routing
struct Cidr {
    addr: IpAddr,
    bits: u32,
}

impl Cidr {
    fn parse(s: &str) -> Result<Cidr, Error> {
        let Some((ip, bits)) = s.split_once('/') else {
            return Err(Error::NoSeparator);
        };

        let addr = IpAddr::from_str(ip).map_err(|_err| Error::Addr)?;

        let bits = bits.parse::<u32>().map_err(|_err| Error::Bits)?;

        // validate bits
        let bits = match addr {
            IpAddr::V4(_) => 32u32.checked_sub(bits),
            IpAddr::V6(_) => 128u32.checked_sub(bits),
        }
        .ok_or(Error::Bits)?;

        Ok(Cidr { addr, bits })
    }

    fn contains(&self, ip: &IpAddr) -> bool {
        match (self.addr, ip) {
            (IpAddr::V4(addr), IpAddr::V4(ip)) => {
                let addr = u32::from_be_bytes(addr.octets()) >> self.bits;
                let ip = u32::from_be_bytes(ip.octets()) >> self.bits;

                addr == ip
            }
            (IpAddr::V6(addr), IpAddr::V6(ip)) => {
                let addr = u128::from_be_bytes(addr.octets()) >> self.bits;
                let ip = u128::from_be_bytes(ip.octets()) >> self.bits;

                addr == ip
            }
            _ => false,
        }
    }
}

pub struct CidrContains;

impl Function for CidrContains {
    fn identifier(&self) -> &'static str {
        "cidr_contains"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "cidr",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "value",
                kind: Kind::BYTES,
                required: true,
            },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let cidr = arguments.get();
        let value = arguments.get();

        Ok(FunctionCall {
            function: Box::new(CidrContainsFunc { cidr, value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct CidrContainsFunc {
    cidr: Spanned<Expr>,
    value: Spanned<Expr>,
}

impl Expression for CidrContainsFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let cidr = self.cidr.resolve(cx)?;
        let Value::Bytes(value) = cidr else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: cidr.kind(),
                span: self.cidr.span,
            });
        };

        let cidr = Cidr::parse(String::from_utf8_lossy(&value).as_ref()).map_err(|err| {
            ExpressionError::UnexpectedValue {
                msg: format!("invalid cidr, {}", err),
                span: self.cidr.span,
            }
        })?;

        let value = self.value.resolve(cx)?;
        let Value::Bytes(value) = value else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            });
        };
        let ip = String::from_utf8_lossy(&value);
        let ip = IpAddr::from_str(ip.as_ref()).map_err(|err| ExpressionError::UnexpectedValue {
            msg: format!("invalid ip address, {}", err),
            span: self.value.span,
        })?;

        Ok(cidr.contains(&ip).into())
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::BOOLEAN,
        }
    }
}

#[cfg(test)]
mod cidr_tests {
    use super::*;
    use std::net::Ipv6Addr;

    #[test]
    fn v4_24bit() {
        let cidr = Cidr::parse("192.0.2.0/24").unwrap();
        let ip = IpAddr::from([0xc0, 0x00, 0x02, 0x01]);

        assert!(cidr.contains(&ip))
    }

    #[test]
    fn v4_not_24bit() {
        let cidr = Cidr::parse("192.0.2.0/24").unwrap();
        let ip = IpAddr::from([0x40, 0x00, 0x02, 0x01]);

        assert!(!cidr.contains(&ip))
    }

    #[test]
    fn v4_not_24bit_2() {
        let cidr = Cidr::parse("192.0.2.0/24").unwrap();
        let ip = IpAddr::from([0xc0, 0x00, 0x03, 0x01]);

        assert!(!cidr.contains(&ip))
    }

    #[test]
    fn v4() {
        let cidr = Cidr::parse("1.2.3.0/30").unwrap();

        for input in ["1.2.3.0", "1.2.3.1", "1.2.3.2", "1.2.3.3"] {
            let ip = input.parse::<IpAddr>().unwrap();
            assert!(cidr.contains(&ip))
        }

        // not
        for input in ["1.2.3.4", "1.2.3.10", "1.2.3.11", "1.2.3.12", "1.2.3.13"] {
            let ip = input.parse::<IpAddr>().unwrap();
            assert!(!cidr.contains(&ip))
        }
    }

    #[test]
    fn v6_64bit() {
        let cidr = Cidr::parse("2001:DB8:1234:5678::/64").unwrap();
        // 2001:0DB8:1234:5678:0000:0000:0000:0000
        let ip = IpAddr::from(Ipv6Addr::new(
            0x2001, 0x0db8, 0x1234, 0x5678, 0x1001, 2, 3, 4,
        ));

        assert!(cidr.contains(&ip))
    }

    #[test]
    fn v6_not_64bit() {
        let cidr = Cidr::parse("2001:DB8:1234:5678::/64").unwrap();
        let ip = IpAddr::from(Ipv6Addr::new(
            0xa001, 0xdb8, 0x1234, 0x5678, 0x1001, 2, 3, 4,
        ));

        assert!(!cidr.contains(&ip))
    }

    #[test]
    fn v6_not_64bit_2() {
        let cidr = Cidr::parse("2001:DB8:1234:5678::/64").unwrap();
        let ip = IpAddr::from(Ipv6Addr::new(
            0xa001, 0xdb8, 0x1234, 0x5679, 0x1001, 2, 3, 4,
        ));

        assert!(!cidr.contains(&ip))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn ipv4_yes() {
        compile_and_run(
            vec!["192.168.0.0/16".into(), "192.168.10.32".into()],
            CidrContains,
            TypeDef::boolean(),
            Ok(Value::Boolean(true)),
        )
    }

    #[test]
    fn ipv4_no() {
        compile_and_run(
            vec!["192.168.0.0/24".into(), "192.168.10.32".into()],
            CidrContains,
            TypeDef::boolean(),
            Ok(Value::Boolean(false)),
        )
    }

    #[test]
    fn ipv6_yes() {
        compile_and_run(
            vec![
                "2001:4f8:3:ba::/64".into(),
                "2001:4f8:3:ba:2e0:81ff:fe22:d1f1".into(),
            ],
            CidrContains,
            TypeDef::boolean(),
            Ok(Value::Boolean(true)),
        )
    }

    #[test]
    fn ipv6_no() {
        compile_and_run(
            vec![
                "2001:4f8:4:ba::/64".into(),
                "2001:4f8:3:ba:2e0:81ff:fe22:d1f1".into(),
            ],
            CidrContains,
            TypeDef::boolean(),
            Ok(Value::Boolean(false)),
        )
    }
}
