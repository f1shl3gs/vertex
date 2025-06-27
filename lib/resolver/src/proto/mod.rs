#![allow(dead_code)]

use std::net::{Ipv4Addr, Ipv6Addr};

/// The type of the resource record
///
/// This specifies the type of data in the RData field of the Resource Record
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordType {
    // ResourceHeader.Type and Question.Type
    A,
    NS,
    CNAME,
    SOA,
    PTR,
    MX,
    TXT,
    AAAA,
    SRV,
    OPT,

    // Question.Type
    WKS,
    HINFO,
    MINFO,
    AXFR,
    ALL,

    Unknown(u16),
}

impl From<u16> for RecordType {
    fn from(value: u16) -> Self {
        match value {
            // ResourceHeader.Type and Question.Type
            1 => RecordType::A,
            2 => RecordType::NS,
            5 => RecordType::CNAME,
            6 => RecordType::SOA,
            12 => RecordType::PTR,
            15 => RecordType::MX,
            16 => RecordType::TXT,
            28 => RecordType::AAAA,
            33 => RecordType::SRV,
            41 => RecordType::OPT,

            // Question.Type
            11 => RecordType::WKS,
            13 => RecordType::HINFO,
            14 => RecordType::MINFO,
            252 => RecordType::AXFR,
            255 => RecordType::ALL,

            _ => RecordType::Unknown(value),
        }
    }
}

impl RecordType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecordType::A => "A",
            RecordType::NS => "NS",
            RecordType::CNAME => "CNAME",
            RecordType::SOA => "SOA",
            RecordType::PTR => "PTR",
            RecordType::MX => "MX",
            RecordType::TXT => "TXT",
            RecordType::AAAA => "AAAA",
            RecordType::SRV => "SRV",
            RecordType::OPT => "OPT",
            RecordType::WKS => "WKS",
            RecordType::HINFO => "HINFO",
            RecordType::MINFO => "MINFO",
            RecordType::AXFR => "AXFR",
            RecordType::ALL => "ALL",
            RecordType::Unknown(_) => "UNKNOWN",
        }
    }

    pub fn to_u16(&self) -> u16 {
        match self {
            // ResourceHeader.Type and Question.Type
            RecordType::A => 1,
            RecordType::NS => 2,
            RecordType::CNAME => 5,
            RecordType::SOA => 6,
            RecordType::PTR => 12,
            RecordType::MX => 15,
            RecordType::TXT => 16,
            RecordType::AAAA => 28,
            RecordType::SRV => 33,
            RecordType::OPT => 41,

            // Question.Type
            RecordType::WKS => 11,
            RecordType::HINFO => 13,
            RecordType::MINFO => 14,
            RecordType::AXFR => 252,
            RecordType::ALL => 255,

            RecordType::Unknown(value) => *value,
        }
    }
}

/// The DNS Record class
#[derive(Clone, Debug, PartialEq)]
pub enum RecordClass {
    /// Internet
    INET,
    CSNET,
    /// Chaos
    CHAOS,
    /// Hesiod
    HESIOD,
    /// QCLASS NONE
    NONE,
    /// QCLASS * (ANY)
    ANY,
    /// Special class for OPT Version, it was overloaded for EDNS - RFC 6891
    /// From the RFC: `Values lower than 512 MUST be treated as equal to 512`
    OPT(u16),
    /// Unknown DNSClass was parsed
    Unknown(u16),
}

impl From<u16> for RecordClass {
    fn from(value: u16) -> Self {
        match value {
            1 => RecordClass::INET,
            2 => RecordClass::CSNET,
            3 => RecordClass::CHAOS,
            4 => RecordClass::HESIOD,
            255 => RecordClass::ANY,
            _ => RecordClass::Unknown(value),
        }
    }
}

impl RecordClass {
    pub fn to_u16(&self) -> u16 {
        match self {
            RecordClass::INET => 1,
            RecordClass::CSNET => 2,
            RecordClass::CHAOS => 3,
            RecordClass::HESIOD => 4,
            RecordClass::NONE => 254,
            RecordClass::ANY => 255,
            RecordClass::OPT(value) => *value,
            RecordClass::Unknown(unknown) => *unknown,
        }
    }
}

/// An RCode is a DNS response status code.
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub enum RCode {
    Success,
    FormatError,
    ServerFailure,
    NameError,
    NotImplemented,
    Refused,

    /// An unknown or unregistered response code was received.
    ///
    /// 24-3840      Unassigned
    /// 3841-4095    Reserved for Private Use                        [RFC6895]
    /// 4096-65534   Unassigned
    /// 65535        Reserved, can be allocated by Standards Action  [RFC6895]
    Unknown(u16),
}

impl From<u16> for RCode {
    fn from(value: u16) -> Self {
        match value {
            0 => RCode::Success,
            1 => RCode::FormatError,
            2 => RCode::ServerFailure,
            3 => RCode::NameError,
            4 => RCode::NotImplemented,
            5 => RCode::Refused,
            _ => RCode::Unknown(value),
        }
    }
}

impl RCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RCode::Success => "Success",
            RCode::FormatError => "FormatError",
            RCode::ServerFailure => "ServerFailure",
            RCode::NameError => "NameError",
            RCode::NotImplemented => "NotImplemented",
            RCode::Refused => "Refused",
            RCode::Unknown(_) => "Unknown",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Header {
    pub id: u16,
    pub flags: u16,

    pub questions: u16,
    pub answers: u16,
    pub authorities: u16,
    pub additionals: u16,
}

impl Header {
    /// A 16 bit identifier assigned by the program that generates any kind of query.
    /// This identifier is copied the corresponding reply and can be used by the requester
    /// to match up replies to outstanding queries.
    #[inline]
    pub fn id(&self) -> u16 {
        self.id
    }

    /// A four bit field that specifies kind of query in this message.  This value
    /// is set by the originator of a query and copied into the response.
    /// The values are: <see super::op_code>
    #[inline]
    pub fn opcode(&self) -> u16 {
        (self.flags >> 11) & 0xF
    }

    /// Authoritative Answer - this bit is valid in responses, and specifies that
    /// the responding name server is an authority for the domain name in question section.
    ///
    /// Note that the contents of the answer section may have multiple owner names
    /// because of aliases.  The AA bit corresponds to the name which matches the
    /// query name, or the first owner name in the answer section.
    #[inline]
    pub fn authoritative(&self) -> bool {
        self.flags & (1 << 10) != 0
    }

    /// TrunCation - specifies that this message was truncated due to length greater
    /// than that permitted on the transmission channel.
    #[inline]
    pub fn truncated(&self) -> bool {
        (self.flags & (1 << 9)) != 0
    }

    /// Recursion Desired - this bit may be set in a query and is copied into the
    /// response.  If RD is set, it directs the name server to pursue the query
    /// recursively. Recursive query support is optional.
    #[inline]
    pub fn recursion_desired(&self) -> bool {
        (self.flags & (1 << 8)) != 0
    }

    /// Recursion Available - this be is set or cleared in a response, and denotes
    /// whether recursive query support is available in the name server.
    #[inline]
    pub fn recursion_available(&self) -> bool {
        (self.flags & (1 << 7)) != 0
    }

    #[inline]
    pub fn zero(&self) -> bool {
        (self.flags & (1 << 6)) != 0
    }

    /// [RFC 4035, DNSSEC Resource Records, March 2005](https://tools.ietf.org/html/rfc4035#section-3.1.6)
    /// The AD and CD Bits in an Authoritative Response
    ///
    /// The CD and AD bits are designed for use in communication between
    /// security-aware resolvers and security-aware recursive name servers.
    /// These bits are for the most part not relevant to query processing by
    /// security-aware authoritative name servers.
    ///
    /// A security-aware name server does not perform signature validation
    /// for authoritative data during query processing, even when the CD bit
    /// is clear.  A security-aware name server SHOULD clear the CD bit when
    /// composing an authoritative response.
    ///
    /// A security-aware name server MUST NOT set the AD bit in a response
    /// unless the name server considers all RRsets in the Answer and
    /// Authority sections of the response to be authentic.  A security-aware
    /// name server's local policy MAY consider data from an authoritative
    /// zone to be authentic without further validation.  However, the name
    /// server MUST NOT do so unless the name server obtained the
    /// authoritative zone via secure means (such as a secure zone transfer
    /// mechanism) and MUST NOT do so unless this behavior has been
    /// configured explicitly.
    ///
    /// A security-aware name server that supports recursion MUST follow the
    /// rules for the CD and AD bits given in Section 3.2 when generating a
    /// response that involves data obtained via recursion.
    #[inline]
    pub fn authenticated_data(&self) -> bool {
        (self.flags & (1 << 5)) != 0
    }

    #[inline]
    pub fn checking_disabled(&self) -> bool {
        (self.flags & (1 << 4)) != 0
    }

    /// Response code - this 4 bit field is set as part of responses. The values
    /// have the following interpretation: <see super::response_code>
    #[inline]
    pub fn response_code(&self) -> RCode {
        RCode::from(self.flags & 0xF)
    }

    /// `QR` A one bit field that specifies whether this message is a query(0) or
    /// response(1)
    pub fn response(&self) -> bool {
        (self.flags & (1 << 15)) != 0
    }
}

/// Maximum TTL. This is set to one day (in seconds).
///
/// [RFC 2181, section 8](https://tools.ietf.org/html/rfc2181#section-8) says
/// that the maximum TTL value is 2147483647, but implementations may place an
/// upper bound on received TTLs.
pub(crate) const MAX_TTL: u32 = 86400_u32;
pub(crate) const HEADER_SIZE: usize = 12;

/// Query struct for looking up resource records, basically a resource record without RDATA.
///
/// The question section is used to carry the "question" in most queries, i.e., the parameters
/// that define what is being asked.
#[derive(Clone, Debug)]
pub struct Question {
    pub name: Vec<u8>,
    pub typ: RecordType,
    pub class: RecordClass,
}

#[allow(clippy::upper_case_acronyms)]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Clone, Debug)]
pub enum RecordData {
    NoData,

    A(Ipv4Addr),
    NS(Vec<u8>),
    CNAME(Vec<u8>),
    SOA {
        ns: Vec<u8>,
        mbox: Vec<u8>,
        serial: u32,
        refresh: u32,
        retry: u32,
        expire: u32,

        // MinTTL is the default TTL of resources records which did not contain a
        // TTL value and the TTL of negative responses
        //
        // RFC 2308 Section 4
        min_ttl: u32,
    },
    PTR(Vec<u8>),
    MX {
        /// A 16-bit integer which specifies the preference given to this RR among
        /// others at the same owner. Lower values are preferred
        preference: u16,

        /// A <domain-name> which specifies a host willing to act as a mail exchange
        /// for the owner name
        exchange: Vec<u8>,
    },
    TXT(Vec<Vec<u8>>),
    AAAA(Ipv6Addr),
    SRV {
        /// The priority of this target host. A client MUST attempt to contact the
        /// target host with the lowest-numbered priority it can reach; target hosts
        /// with the same priority SHOULD be tried in an order defined by the weight
        /// field.
        ///
        /// The range is 0-65535. This is a 16-bit unsigned integer in network order.
        priority: u16,

        /// A server selection mechanism. The weight field specifies a relative weight
        /// for entries with the same priority. Larger weights SHOULD be given a
        /// proportionately higher probability of being selected.
        ///
        /// The range of this number if 0-65535, 16-bit unsigned integer in network order.
        ///
        /// Domain administrators SHOULD use Weight 0 when there isn't any server
        /// selection to do, to make the RR easier to read for humans (less noisy).
        weight: u16,

        /// The port on this target host of this service.
        ///
        /// The range is 0-65535. This is a 16-bit unsigned integer in network order
        /// This is often as specified in Assigned Numbers but need not be.
        port: u16,

        /// The domain name of the target host. There MUST be one or more address
        /// records for this name, the name MUST NOT be an alias (in the sense of
        /// RFC 1034 or RFC 2181). Implementors are urged, but not required, to
        /// return the address record(s) in the Additional Data section. Unless and
        /// until permitted by future standards action, name compression is not to
        /// be used for this field.
        ///
        /// A target of "." means that the service is decidedly not available at
        /// this domain.
        target: Vec<u8>,
    },
    OPT(Vec<Opt>),

    Unknown {
        typ: RecordType,
        data: Vec<u8>,
    },
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Clone, Debug)]
pub struct Opt {
    pub code: u16,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Record {
    pub name: Vec<u8>,
    pub typ: RecordType,
    pub class: RecordClass,
    pub ttl: u32,
    pub data: RecordData,
}

#[derive(Clone, Debug)]
pub struct Message {
    pub header: Header,

    pub questions: Vec<Question>,
    pub answers: Vec<Record>,
    pub authorities: Vec<Record>,
    pub additionals: Vec<Record>,
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Clone, Debug)]
pub enum Error {
    /// buffer is too small
    TooSmall,

    /// Too many compression pointers
    TooManyCompressionPointers,

    /// Invalid record data
    InvalidRecordData,

    /// domain name exceeded 255 wire-format octets
    LongDomain,

    InvalidTTL(u32),
}

/// See RFC 1035 section 2.3.4
const MAX_DOMAIN_NAME_WIRE_OCTETS: usize = 255;

/// This is the maximum number of compression pointers that should occur in a
/// semantically valid message. Each label in a domain name must be at least one
/// octet and is separated by a period.
const MAX_COMPRESSION_POINTERS: usize = 10;

fn decode_name(buf: &[u8], start: &mut usize) -> Result<Vec<u8>, Error> {
    let mut name = Vec::<u8>::with_capacity(32);

    let mut pointers = 0;
    let mut pos = *start;
    loop {
        if pos >= buf.len() {
            return Err(Error::TooSmall);
        }

        let len = buf[pos] as usize;
        pos += 1;
        match len & 0xc0 {
            0x00 => {
                if len == 0 {
                    break;
                }

                // +1 for the label separator
                if name.len() + 1 > MAX_DOMAIN_NAME_WIRE_OCTETS {
                    return Err(Error::LongDomain);
                }

                if pos + len > buf.len() {
                    return Err(Error::TooSmall);
                }

                // name.extend_from_slice(&buf[pos..pos + len]);

                name.reserve(len + 1);
                let nl = name.len();
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        buf.as_ptr().add(pos),
                        name.as_mut_ptr().add(nl),
                        len,
                    );
                    name.set_len(nl + len);
                }

                name.push(b'.');
                pos += len;
            }
            0xc0 => {
                // pointer
                if pointers == 0 {
                    *start = pos + 1;
                }

                pointers += 1;
                if pointers > MAX_COMPRESSION_POINTERS {
                    return Err(Error::TooManyCompressionPointers);
                }

                pos = ((len ^ 0xc0) << 8) | buf[pos] as usize;
                continue;
            }
            _ => {
                // 0x80 and 0x40 are reserved
                return Err(Error::InvalidRecordData);
            }
        }
    }

    if name.is_empty() {
        name.push(b'.');
    }

    if pointers == 0 {
        *start = pos;
    }

    Ok(name)
}

pub fn decode_message(buf: &[u8]) -> Result<Message, Error> {
    assert!(buf.len() > HEADER_SIZE);

    let header = Header {
        id: ((buf[0] as u16) << 8) | buf[1] as u16,
        flags: ((buf[2] as u16) << 8) | buf[3] as u16,
        questions: ((buf[4] as u16) << 8) | buf[5] as u16,
        answers: ((buf[6] as u16) << 8) | buf[7] as u16,
        authorities: ((buf[8] as u16) << 8) | buf[9] as u16,
        additionals: ((buf[10] as u16) << 8) | buf[11] as u16,
    };

    let mut pos = HEADER_SIZE;

    let mut questions = Vec::with_capacity(header.questions as usize);
    for _ in 0..header.questions {
        let name = decode_name(buf, &mut pos)?;
        let typ = RecordType::from(((buf[pos] as u16) << 8) | buf[pos + 1] as u16);
        let class = RecordClass::from(((buf[pos + 2] as u16) << 8) | buf[pos + 3] as u16);
        pos += 4;

        questions.push(Question { name, typ, class });
    }

    let answers = decode_records(header.answers, buf, &mut pos)?;
    let authorities = decode_records(header.authorities, buf, &mut pos)?;
    let additionals = decode_records(header.additionals, buf, &mut pos)?;

    Ok(Message {
        header,
        questions,
        answers,
        authorities,
        additionals,
    })
}

fn decode_records(count: u16, buf: &[u8], pos: &mut usize) -> Result<Vec<Record>, Error> {
    let mut records = Vec::with_capacity(count as usize);

    for _ in 0..count {
        let name = decode_name(buf, pos)?;
        let typ = RecordType::from(((buf[*pos] as u16) << 8) | buf[*pos + 1] as u16);
        let value = ((buf[*pos + 2] as u16) << 8) | buf[*pos + 3] as u16;
        let class = if typ == RecordType::OPT {
            RecordClass::OPT(value)
        } else {
            RecordClass::from(value)
        };
        let ttl = u32::from_be_bytes(buf[*pos + 4..*pos + 8].try_into().unwrap());

        let rdlen = u16::from_be_bytes(buf[*pos + 8..*pos + 10].try_into().unwrap()) as usize;
        *pos += 10;

        let data = if rdlen == 0 {
            RecordData::NoData
        } else {
            let reloc = *pos + rdlen;

            let data = match typ {
                RecordType::A => {
                    if buf.len() - *pos < 4 {
                        return Err(Error::TooSmall);
                    }

                    RecordData::A(Ipv4Addr::new(
                        buf[*pos],
                        buf[*pos + 1],
                        buf[*pos + 2],
                        buf[*pos + 3],
                    ))
                }
                RecordType::NS => RecordData::NS(decode_name(buf, pos)?),
                RecordType::CNAME => RecordData::CNAME(decode_name(buf, pos)?),
                RecordType::SOA => {
                    let ns = decode_name(buf, pos)?;
                    let mbox = decode_name(buf, pos)?;

                    if buf.len() - *pos < 20 {
                        return Err(Error::TooSmall);
                    }

                    let serial = ((buf[*pos] as u32) << 24)
                        | ((buf[*pos + 1] as u32) << 16)
                        | ((buf[*pos + 2] as u32) << 8)
                        | buf[*pos + 3] as u32;
                    let refresh = ((buf[*pos + 4] as u32) << 24)
                        | ((buf[*pos + 5] as u32) << 16)
                        | ((buf[*pos + 6] as u32) << 8)
                        | buf[*pos + 7] as u32;
                    let retry = ((buf[*pos + 8] as u32) << 24)
                        | ((buf[*pos + 9] as u32) << 16)
                        | ((buf[*pos + 10] as u32) << 8)
                        | buf[*pos + 11] as u32;
                    let expire = ((buf[*pos + 12] as u32) << 24)
                        | ((buf[*pos + 13] as u32) << 16)
                        | ((buf[*pos + 14] as u32) << 8)
                        | buf[*pos + 15] as u32;
                    let min_ttl = ((buf[*pos + 16] as u32) << 24)
                        | ((buf[*pos + 17] as u32) << 16)
                        | ((buf[*pos + 18] as u32) << 8)
                        | buf[*pos + 19] as u32;

                    RecordData::SOA {
                        ns,
                        mbox,
                        serial,
                        refresh,
                        retry,
                        expire,
                        min_ttl,
                    }
                }
                RecordType::PTR => RecordData::PTR(decode_name(buf, pos)?),
                RecordType::MX => {
                    let preference = ((buf[*pos] as u16) << 8) | buf[*pos + 1] as u16;
                    *pos += 2;

                    RecordData::MX {
                        preference,
                        exchange: decode_name(buf, pos)?,
                    }
                }
                RecordType::TXT => {
                    let mut fields = Vec::new();
                    while *pos < reloc {
                        let len = buf[*pos] as usize;
                        *pos += 1;

                        fields.push(buf[*pos..*pos + len].to_vec());
                        *pos += len;
                    }

                    RecordData::TXT(fields)
                }
                RecordType::AAAA => {
                    if buf.len() - *pos < 16 {
                        return Err(Error::InvalidRecordData);
                    }

                    RecordData::AAAA(Ipv6Addr::from([
                        buf[*pos],
                        buf[*pos + 1],
                        buf[*pos + 2],
                        buf[*pos + 3],
                        buf[*pos + 4],
                        buf[*pos + 5],
                        buf[*pos + 6],
                        buf[*pos + 7],
                        buf[*pos + 8],
                        buf[*pos + 9],
                        buf[*pos + 10],
                        buf[*pos + 11],
                        buf[*pos + 12],
                        buf[*pos + 13],
                        buf[*pos + 14],
                        buf[*pos + 15],
                    ]))
                }
                RecordType::SRV => {
                    let priority = ((buf[*pos] as u16) << 8) | buf[*pos + 1] as u16;
                    let weight = ((buf[*pos + 2] as u16) << 8) | buf[*pos + 3] as u16;
                    let port = ((buf[*pos + 4] as u16) << 8) | buf[*pos + 5] as u16;
                    *pos += 6;

                    RecordData::SRV {
                        priority,
                        weight,
                        port,
                        // name compression is not to be used for this field.
                        target: decode_name(buf, pos)?,
                    }
                }
                RecordType::OPT => {
                    let mut options = Vec::new();

                    while *pos < reloc {
                        let code = ((buf[*pos] as u16) << 8) | buf[*pos + 1] as u16;
                        let len = ((buf[*pos + 2] as u16) << 8) | buf[*pos + 3] as u16;
                        *pos += 4;

                        let data = Vec::from(&buf[*pos..*pos + len as usize]);
                        *pos += len as usize;

                        options.push(Opt { code, data });
                    }

                    RecordData::OPT(options)
                }
                _ => RecordData::Unknown {
                    typ,
                    data: buf[*pos..*pos + rdlen].to_vec(),
                },
            };

            *pos = reloc;

            data
        };

        records.push(Record {
            name,
            typ,
            class,
            ttl,
            data,
        });
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_resp_with_additionals() {
        /*
        dig www.sina.com

        ; <<>> DiG 9.18.36 <<>> www.sina.com
        ;; global options: +cmd
        ;; Got answer:
        ;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 29774
        ;; flags: qr rd ra; QUERY: 1, ANSWER: 18, AUTHORITY: 0, ADDITIONAL: 1

        ;; OPT PSEUDOSECTION:
        ; EDNS: version: 0, flags:; udp: 65494
        ;; QUESTION SECTION:
        ;www.sina.com.                  IN      A

        ;; ANSWER SECTION:
        www.sina.com.           79      IN      CNAME   spool.grid.sinaedge.com.
        spool.grid.sinaedge.com. 53     IN      CNAME   ww1.sinaimg.cn.w.alikunlun.com.
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       115.231.187.31
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       220.185.164.221
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       220.185.164.206
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       220.185.164.223
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       122.225.215.235
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       122.225.215.234
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       122.225.215.237
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       122.225.215.233
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       220.185.164.224
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       115.231.187.30
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       115.231.187.26
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       115.231.187.33
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       115.231.187.27
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       115.231.187.32
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       115.231.187.29
        ww1.sinaimg.cn.w.alikunlun.com. 218 IN  A       115.231.187.28

        ;; Query time: 6 msec
        ;; SERVER: 127.0.0.53#53(127.0.0.53) (UDP)
        ;; WHEN: Sat May 17 01:45:43 CST 2025
        ;; MSG SIZE  rcvd: 372

        */

        // data from tcpdump
        let input: [u8; 372] = [
            0x74, 0x4e, 0x81, 0x80, 0x00, 0x01, 0x00, 0x12, 0x00, 0x00, 0x00, 0x01, 0x03, 0x77,
            0x77, 0x77, 0x04, 0x73, 0x69, 0x6e, 0x61, 0x03, 0x63, 0x6f, 0x6d, 0x00, 0x00, 0x01,
            0x00, 0x01, 0xc0, 0x0c, 0x00, 0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x4f, 0x00, 0x16,
            0x05, 0x73, 0x70, 0x6f, 0x6f, 0x6c, 0x04, 0x67, 0x72, 0x69, 0x64, 0x08, 0x73, 0x69,
            0x6e, 0x61, 0x65, 0x64, 0x67, 0x65, 0xc0, 0x15, 0xc0, 0x2a, 0x00, 0x05, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x35, 0x00, 0x1d, 0x03, 0x77, 0x77, 0x31, 0x07, 0x73, 0x69, 0x6e,
            0x61, 0x69, 0x6d, 0x67, 0x02, 0x63, 0x6e, 0x01, 0x77, 0x09, 0x61, 0x6c, 0x69, 0x6b,
            0x75, 0x6e, 0x6c, 0x75, 0x6e, 0xc0, 0x15, 0xc0, 0x4c, 0x00, 0x01, 0x00, 0x01, 0x00,
            0x00, 0x00, 0xda, 0x00, 0x04, 0x73, 0xe7, 0xbb, 0x1f, 0xc0, 0x4c, 0x00, 0x01, 0x00,
            0x01, 0x00, 0x00, 0x00, 0xda, 0x00, 0x04, 0xdc, 0xb9, 0xa4, 0xdd, 0xc0, 0x4c, 0x00,
            0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0xda, 0x00, 0x04, 0xdc, 0xb9, 0xa4, 0xce, 0xc0,
            0x4c, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0xda, 0x00, 0x04, 0xdc, 0xb9, 0xa4,
            0xdf, 0xc0, 0x4c, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0xda, 0x00, 0x04, 0x7a,
            0xe1, 0xd7, 0xeb, 0xc0, 0x4c, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0xda, 0x00,
            0x04, 0x7a, 0xe1, 0xd7, 0xea, 0xc0, 0x4c, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00,
            0xda, 0x00, 0x04, 0x7a, 0xe1, 0xd7, 0xed, 0xc0, 0x4c, 0x00, 0x01, 0x00, 0x01, 0x00,
            0x00, 0x00, 0xda, 0x00, 0x04, 0x7a, 0xe1, 0xd7, 0xe9, 0xc0, 0x4c, 0x00, 0x01, 0x00,
            0x01, 0x00, 0x00, 0x00, 0xda, 0x00, 0x04, 0xdc, 0xb9, 0xa4, 0xe0, 0xc0, 0x4c, 0x00,
            0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0xda, 0x00, 0x04, 0x73, 0xe7, 0xbb, 0x1e, 0xc0,
            0x4c, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0xda, 0x00, 0x04, 0x73, 0xe7, 0xbb,
            0x1a, 0xc0, 0x4c, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0xda, 0x00, 0x04, 0x73,
            0xe7, 0xbb, 0x21, 0xc0, 0x4c, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0xda, 0x00,
            0x04, 0x73, 0xe7, 0xbb, 0x1b, 0xc0, 0x4c, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00,
            0xda, 0x00, 0x04, 0x73, 0xe7, 0xbb, 0x20, 0xc0, 0x4c, 0x00, 0x01, 0x00, 0x01, 0x00,
            0x00, 0x00, 0xda, 0x00, 0x04, 0x73, 0xe7, 0xbb, 0x1d, 0xc0, 0x4c, 0x00, 0x01, 0x00,
            0x01, 0x00, 0x00, 0x00, 0xda, 0x00, 0x04, 0x73, 0xe7, 0xbb, 0x1c, 0x00, 0x00, 0x29,
            0xff, 0xd6, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let _msg = decode_message(input.as_ref()).unwrap();
    }

    #[test]
    fn unpack_name() {
        for (label, input, want) in [
            (
                "empty domain",
                "\x00".as_bytes(),
                Ok(b".".to_vec()),
            ),
            (
                "long label",
                "\x3fabcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789x\x00".as_bytes(),
                Ok("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789x.".as_bytes().to_vec()),
            ),
            (
                "long domain",
                &[53, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 48, 49, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 49, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 49, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 49, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 0],
                Ok("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0.abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVW.abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVW.abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVW.abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVW.".as_bytes().to_vec()),
            ),
            // (
            //     "compression pointer",
            //     &[3, 102, 111, 111, 5, 3, 99, 111, 109, 0, 7, 101, 120, 97, 109, 112, 108, 101, 192, 5],
            //     // "foo.\\003com\\000.example.com.".as_bytes(),
            //     Ok(vec![102, 111, 111, 46, 92, 48, 48, 51, 99, 111, 109, 92, 48, 48, 48, 46, 101, 120, 97, 109, 112, 108, 101, 46, 99, 111, 109, 46]),
            // ),
            // (
            //     "long by pointer",
            //     &[37, 34, 31, 28, 25, 22, 19, 16, 13, 10, 0, 120, 120, 120, 120, 120, 120, 120, 120, 120, 192, 10, 192, 9, 192, 8, 192, 7, 192, 6, 192, 5, 192, 4, 192, 3, 192, 2, 192, 1,],
            //     &[92, 34, 92, 48, 51, 49, 92, 48, 50, 56, 92, 48, 50, 53, 92, 48, 50, 50, 92, 48, 49, 57, 92, 48, 49, 54, 92, 48, 49, 51, 92, 48, 49, 48, 92, 48, 48, 48, 120, 120, 120, 120, 120, 120, 120, 120, 120, 92, 49, 57, 50, 92, 48, 49, 48, 92, 49, 57, 50, 92, 48, 48, 57, 92, 49, 57, 50, 92, 48, 48, 56, 92, 49, 57, 50, 92, 48, 48, 55, 92, 49, 57, 50, 92, 48, 48, 54, 92, 49, 57, 50, 92, 48, 48, 53, 92, 49, 57, 50, 92, 48, 48, 52, 92, 49, 57, 50, 92, 48, 48, 51, 92, 49, 57, 50, 92, 48, 48, 50, 46, 92, 48, 51, 49, 92, 48, 50, 56, 92, 48, 50, 53, 92, 48, 50, 50, 92, 48, 49, 57, 92, 48, 49, 54, 92, 48, 49, 51, 92, 48, 49, 48, 92, 48, 48, 48, 120, 120, 120, 120, 120, 120, 120, 120, 120, 92, 49, 57, 50, 92, 48, 49, 48, 92, 49, 57, 50, 92, 48, 48, 57, 92, 49, 57, 50, 92, 48, 48, 56, 92, 49, 57, 50, 92, 48, 48, 55, 92, 49, 57, 50, 92, 48, 48, 54, 92, 49, 57, 50, 92, 48, 48, 53, 92, 49, 57, 50, 92, 48, 48, 52, 92, 49, 57, 50, 92, 48, 48, 51, 46, 92, 48, 50, 56, 92, 48, 50, 53, 92, 48, 50, 50, 92, 48, 49, 57, 92, 48, 49, 54, 92, 48, 49, 51, 92, 48, 49, 48, 92, 48, 48, 48, 120, 120, 120, 120, 120, 120, 120, 120, 120, 92, 49, 57, 50, 92, 48, 49, 48, 92, 49, 57, 50, 92, 48, 48, 57, 92, 49, 57, 50, 92, 48, 48, 56, 92, 49, 57, 50, 92, 48, 48, 55, 92, 49, 57, 50, 92, 48, 48, 54, 92, 49, 57, 50, 92, 48, 48, 53, 92, 49, 57, 50, 92, 48, 48, 52, 46, 92, 48, 50, 53, 92, 48, 50, 50, 92, 48, 49, 57, 92, 48, 49, 54, 92, 48, 49, 51, 92, 48, 49, 48, 92, 48, 48, 48, 120, 120, 120, 120, 120, 120, 120, 120, 120, 92, 49, 57, 50, 92, 48, 49, 48, 92, 49, 57, 50, 92, 48, 48, 57, 92, 49, 57, 50, 92, 48, 48, 56, 92, 49, 57, 50, 92, 48, 48, 55, 92, 49, 57, 50, 92, 48, 48, 54, 92, 49, 57, 50, 92, 48, 48, 53, 46, 92, 48, 50, 50, 92, 48, 49, 57, 92, 48, 49, 54, 92, 48, 49, 51, 92, 48, 49, 48, 92, 48, 48, 48, 120, 120, 120, 120, 120, 120, 120, 120, 120, 92, 49, 57, 50, 92, 48, 49, 48, 92, 49, 57, 50, 92, 48, 48, 57, 92, 49, 57, 50, 92, 48, 48, 56, 92, 49, 57, 50, 92, 48, 48, 55, 92, 49, 57, 50, 92, 48, 48, 54, 46, 92, 48, 49, 57, 92, 48, 49, 54, 92, 48, 49, 51, 92, 48, 49, 48, 92, 48, 48, 48, 120, 120, 120, 120, 120, 120, 120, 120, 120, 92, 49, 57, 50, 92, 48, 49, 48, 92, 49, 57, 50, 92, 48, 48, 57, 92, 49, 57, 50, 92, 48, 48, 56, 92, 49, 57, 50, 92, 48, 48, 55, 46, 92, 48, 49, 54, 92, 48, 49, 51, 92, 48, 49, 48, 92, 48, 48, 48, 120, 120, 120, 120, 120, 120, 120, 120, 120, 92, 49, 57, 50, 92, 48, 49, 48, 92, 49, 57, 50, 92, 48, 48, 57, 92, 49, 57, 50, 92, 48, 48, 56, 46, 92, 48, 49, 51, 92, 48, 49, 48, 92, 48, 48, 48, 120, 120, 120, 120, 120, 120, 120, 120, 120, 92, 49, 57, 50, 92, 48, 49, 48, 92, 49, 57, 50, 92, 48, 48, 57, 46, 92, 48, 49, 48, 92, 48, 48, 48, 120, 120, 120, 120, 120, 120, 120, 120, 120, 92, 49, 57, 50, 92, 48, 49, 48, 46, 92, 48, 48, 48, 120, 120, 120, 120, 120, 120, 120, 120, 120, 46,],
            // )
        ] {
            let mut pos = 0;
            let got = decode_name(input, &mut pos);

            assert_eq!(got, want, "label: {label}");
        }
    }

    #[test]
    fn large_message() {
        let input = [
            0, 0, 132, 0, 0, 1, 0, 9, 0, 2, 0, 3, 3, 102, 111, 111, 3, 98, 97, 114, 7, 101, 120,
            97, 109, 112, 108, 101, 3, 99, 111, 109, 0, 0, 1, 0, 1, 192, 12, 0, 1, 0, 1, 0, 0, 0,
            0, 0, 4, 127, 0, 0, 1, 192, 12, 0, 1, 0, 1, 0, 0, 0, 0, 0, 4, 127, 0, 0, 2, 192, 12, 0,
            28, 0, 1, 0, 0, 0, 0, 0, 16, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            192, 12, 0, 5, 0, 1, 0, 0, 0, 0, 0, 8, 5, 97, 108, 105, 97, 115, 192, 20, 192, 12, 0,
            6, 0, 1, 0, 0, 0, 0, 0, 31, 3, 110, 115, 49, 192, 20, 2, 109, 98, 192, 20, 0, 0, 0, 1,
            0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 5, 192, 12, 0, 12, 0, 1, 0, 0, 0, 0, 0, 6,
            3, 112, 116, 114, 192, 20, 192, 12, 0, 15, 0, 1, 0, 0, 0, 0, 0, 7, 0, 7, 2, 109, 120,
            192, 20, 192, 12, 0, 33, 0, 1, 0, 0, 0, 0, 0, 23, 0, 8, 0, 9, 0, 11, 3, 115, 114, 118,
            7, 101, 120, 97, 109, 112, 108, 101, 3, 99, 111, 109, 0, 192, 12, 255, 82, 0, 1, 0, 0,
            0, 0, 0, 4, 42, 0, 43, 44, 192, 12, 0, 2, 0, 1, 0, 0, 0, 0, 0, 2, 192, 129, 192, 12, 0,
            2, 0, 1, 0, 0, 0, 0, 0, 6, 3, 110, 115, 50, 192, 20, 192, 12, 0, 16, 0, 1, 0, 0, 0, 0,
            0, 37, 36, 83, 111, 32, 76, 111, 110, 103, 44, 32, 97, 110, 100, 32, 84, 104, 97, 110,
            107, 115, 32, 102, 111, 114, 32, 65, 108, 108, 32, 116, 104, 101, 32, 70, 105, 115,
            104, 192, 12, 0, 16, 0, 1, 0, 0, 0, 0, 0, 36, 35, 72, 97, 109, 115, 116, 101, 114, 32,
            72, 117, 101, 121, 32, 97, 110, 100, 32, 116, 104, 101, 32, 71, 111, 111, 101, 121, 32,
            75, 97, 98, 108, 111, 111, 105, 101, 0, 0, 41, 16, 0, 254, 0, 0, 0, 0, 12, 0, 10, 0, 8,
            1, 35, 69, 103, 137, 171, 205, 239,
        ];

        let mut msg = decode_message(&input).unwrap();
        // println!("{:#?}", msg);

        let name = "foo.bar.example.com.".as_bytes().to_vec();

        let header = msg.header;
        assert_eq!(header.id, 0);
        assert!(header.response());
        assert_eq!(header.opcode(), 0);
        assert!(header.authoritative());
        assert!(!header.truncated());
        assert!(!header.recursion_desired());
        assert!(!header.recursion_available());
        assert_eq!(header.response_code(), RCode::Success);

        let question = msg.questions.remove(0);
        assert_eq!(question.name, name);
        assert_eq!(question.typ, RecordType::A);
        assert_eq!(question.class, RecordClass::INET);

        let answer = msg.answers.remove(0);
        assert_eq!(answer.name, name);
        assert_eq!(answer.typ, RecordType::A);
        assert_eq!(answer.class, RecordClass::INET);
        assert_eq!(answer.ttl, 0);
        assert_eq!(answer.data, RecordData::A(Ipv4Addr::new(127, 0, 0, 1)));

        let answer = msg.answers.remove(0);
        assert_eq!(answer.name, name);
        assert_eq!(answer.typ, RecordType::A);
        assert_eq!(answer.class, RecordClass::INET);
        assert_eq!(answer.ttl, 0);
        assert_eq!(answer.data, RecordData::A(Ipv4Addr::new(127, 0, 0, 2)));

        let answer = msg.answers.remove(0);
        assert_eq!(answer.name, name);
        assert_eq!(answer.typ, RecordType::AAAA);
        assert_eq!(answer.class, RecordClass::INET);
        assert_eq!(answer.ttl, 0);
        assert_eq!(
            answer.data,
            RecordData::AAAA(Ipv6Addr::from([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16
            ]))
        );

        let answer = msg.answers.remove(0);
        assert_eq!(answer.name, name);
        assert_eq!(answer.typ, RecordType::CNAME);
        assert_eq!(answer.class, RecordClass::INET);
        assert_eq!(answer.ttl, 0);
        assert_eq!(
            answer.data,
            RecordData::CNAME("alias.example.com.".as_bytes().to_vec())
        );

        let answer = msg.answers.remove(0);
        assert_eq!(answer.name, name);
        assert_eq!(answer.typ, RecordType::SOA);
        assert_eq!(answer.class, RecordClass::INET);
        assert_eq!(answer.ttl, 0);
        assert_eq!(
            answer.data,
            RecordData::SOA {
                ns: "ns1.example.com.".as_bytes().to_vec(),
                mbox: "mb.example.com.".as_bytes().to_vec(),
                serial: 1,
                refresh: 2,
                retry: 3,
                expire: 4,
                min_ttl: 5,
            }
        );

        let answer = msg.answers.remove(0);
        assert_eq!(answer.name, name);
        assert_eq!(answer.typ, RecordType::PTR);
        assert_eq!(answer.class, RecordClass::INET);
        assert_eq!(answer.ttl, 0);
        assert_eq!(
            answer.data,
            RecordData::PTR("ptr.example.com.".as_bytes().to_vec())
        );

        let answer = msg.answers.remove(0);
        assert_eq!(answer.name, name);
        assert_eq!(answer.typ, RecordType::MX);
        assert_eq!(answer.class, RecordClass::INET);
        assert_eq!(answer.ttl, 0);
        assert_eq!(
            answer.data,
            RecordData::MX {
                preference: 7,
                exchange: "mx.example.com.".as_bytes().to_vec(),
            }
        );

        let answer = msg.answers.remove(0);
        assert_eq!(answer.name, name);
        assert_eq!(answer.typ, RecordType::SRV);
        assert_eq!(answer.class, RecordClass::INET);
        assert_eq!(answer.ttl, 0);
        assert_eq!(
            answer.data,
            RecordData::SRV {
                priority: 8,
                weight: 9,
                port: 11,
                target: "srv.example.com.".as_bytes().to_vec(),
            }
        );

        let answer = msg.answers.remove(0);
        assert_eq!(answer.name, name);
        assert_eq!(answer.typ, RecordType::Unknown(65362));
        assert_eq!(answer.class, RecordClass::INET);
        assert_eq!(answer.ttl, 0);
        assert_eq!(
            answer.data,
            RecordData::Unknown {
                typ: RecordType::Unknown(65362),
                data: vec![42, 0, 43, 44]
            }
        );

        let authority = msg.authorities.remove(0);
        assert_eq!(authority.name, name);
        assert_eq!(authority.typ, RecordType::NS);
        assert_eq!(authority.class, RecordClass::INET);
        assert_eq!(authority.ttl, 0);
        assert_eq!(
            authority.data,
            RecordData::NS("ns1.example.com.".as_bytes().to_vec())
        );

        let authority = msg.authorities.remove(0);
        assert_eq!(authority.name, name);
        assert_eq!(authority.typ, RecordType::NS);
        assert_eq!(authority.class, RecordClass::INET);
        assert_eq!(authority.ttl, 0);
        assert_eq!(
            authority.data,
            RecordData::NS("ns2.example.com.".as_bytes().to_vec())
        );

        let additional = msg.additionals.remove(0);
        assert_eq!(additional.name, name);
        assert_eq!(additional.typ, RecordType::TXT);
        assert_eq!(additional.class, RecordClass::INET);
        assert_eq!(additional.ttl, 0);
        assert_eq!(
            additional.data,
            RecordData::TXT(vec![
                "So Long\x2c and Thanks for All the Fish"
                    .as_bytes()
                    .to_vec()
            ])
        );

        let additional = msg.additionals.remove(0);
        assert_eq!(additional.name, name);
        assert_eq!(additional.typ, RecordType::TXT);
        assert_eq!(additional.class, RecordClass::INET);
        assert_eq!(additional.ttl, 0);
        assert_eq!(
            additional.data,
            RecordData::TXT(vec![
                "Hamster Huey and the Gooey Kablooie".as_bytes().to_vec(),
            ])
        );

        let additional = msg.additionals.remove(0);
        assert_eq!(additional.name, ".".as_bytes().to_vec());
        assert_eq!(additional.typ, RecordType::OPT);
        assert_eq!(additional.class, RecordClass::OPT(4096));
        assert_eq!(additional.ttl, 4261412864);
        assert_eq!(
            additional.data,
            RecordData::OPT(vec![Opt {
                code: 10,
                data: vec![1, 35, 69, 103, 137, 171, 205, 239]
            }])
        );
    }

    #[test]
    fn small_message() {
        let input = [
            0, 0, 132, 0, 0, 1, 0, 1, 0, 1, 0, 1, 7, 101, 120, 97, 109, 112, 108, 101, 3, 99, 111,
            109, 0, 0, 1, 0, 1, 192, 12, 0, 1, 0, 1, 0, 0, 0, 0, 0, 4, 127, 0, 0, 1, 192, 12, 0, 1,
            0, 1, 0, 0, 0, 0, 0, 4, 127, 0, 0, 1, 192, 12, 0, 1, 0, 1, 0, 0, 0, 0, 0, 4, 127, 0, 0,
            1,
        ];

        let mut msg = decode_message(&input).unwrap();
        let name = "example.com.".as_bytes().to_vec();

        assert!(msg.header.response());
        assert!(msg.header.authoritative());

        let question = msg.questions.remove(0);
        assert_eq!(question.name, name);
        assert_eq!(question.typ, RecordType::A);
        assert_eq!(question.class, RecordClass::INET);

        let answer = msg.answers.remove(0);
        assert_eq!(answer.name, name);
        assert_eq!(answer.typ, RecordType::A);
        assert_eq!(answer.class, RecordClass::INET);
        assert_eq!(answer.ttl, 0);
        assert_eq!(answer.data, RecordData::A(Ipv4Addr::new(127, 0, 0, 1)));

        let authorities = msg.authorities.remove(0);
        assert_eq!(authorities.name, name);
        assert_eq!(authorities.typ, RecordType::A);
        assert_eq!(authorities.class, RecordClass::INET);
        assert_eq!(authorities.ttl, 0);
        assert_eq!(authorities.data, RecordData::A(Ipv4Addr::new(127, 0, 0, 1)));

        let additional = msg.additionals.remove(0);
        assert_eq!(additional.name, name);
        assert_eq!(additional.typ, RecordType::A);
        assert_eq!(additional.class, RecordClass::INET);
        assert_eq!(additional.ttl, 0);
        assert_eq!(additional.data, RecordData::A(Ipv4Addr::new(127, 0, 0, 1)));
    }
}
