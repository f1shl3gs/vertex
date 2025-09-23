use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;

use crate::proto::{Record, RecordClass, RecordData, RecordType};

#[derive(Default, Debug)]
pub struct Records {
    v4: Vec<Record>,
    v6: Vec<Record>,
}

#[derive(Debug, Default)]
pub struct Hosts {
    items: HashMap<String, Records>,
}

impl Hosts {
    pub fn load(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let file = std::fs::File::open(path.as_ref())?;
        // let mtime = file.metadata()?.modified()?;

        let mut items = HashMap::new();
        let mut lines = BufReader::new(file).lines();
        while let Some(Ok(line)) = lines.next() {
            let line = match line.split_once('#') {
                Some((line, _)) => line.trim(),
                None => line.trim(),
            };
            if line.is_empty() {
                continue;
            }

            let mut fields = line.split_ascii_whitespace();
            let Some(first) = fields.next() else {
                continue;
            };

            let first = match first.split_once('%') {
                Some((first, _)) => first,
                None => first,
            };
            let Ok(addr) = IpAddr::from_str(first) else {
                continue;
            };

            for name in fields {
                let entry = items
                    // we don't store the absolute domain name(e.g. `foo.bar.`), cause
                    // the user side do not use that too
                    .entry(name.to_ascii_lowercase())
                    .or_insert(Records::default());

                match addr {
                    IpAddr::V4(addr) => {
                        entry.v4.push(Record {
                            name: encode_name(name),
                            typ: RecordType::A,
                            class: RecordClass::INET,
                            ttl: 0,
                            data: RecordData::A(addr),
                        });
                    }
                    IpAddr::V6(addr) => {
                        entry.v6.push(Record {
                            name: encode_name(name),
                            typ: RecordType::AAAA,
                            class: RecordClass::INET,
                            ttl: 0,
                            data: RecordData::AAAA(addr),
                        });
                    }
                }
            }
        }

        Ok(Hosts { items })
    }

    #[inline]
    pub fn lookup_ipv4(&self, name: &str) -> Option<Vec<Record>> {
        match to_ascii_lowercase(name) {
            Cow::Borrowed(name) => self
                .items
                .get(name.trim_end_matches('.'))
                .map(|records| records.v4.clone()),
            Cow::Owned(name) => self
                .items
                .get(name.trim_end_matches('.'))
                .map(|records| records.v4.clone()),
        }
    }

    #[inline]
    pub fn lookup_ipv6(&self, name: &str) -> Option<Vec<Record>> {
        match to_ascii_lowercase(name) {
            Cow::Borrowed(name) => self
                .items
                .get(name.trim_end_matches('.'))
                .map(|records| records.v6.clone()),
            Cow::Owned(name) => self
                .items
                .get(name.trim_end_matches('.'))
                .map(|records| records.v6.clone()),
        }
    }
}

fn to_ascii_lowercase(name: &str) -> Cow<'_, str> {
    if name.as_bytes().iter().any(|c| c.is_ascii_uppercase()) {
        return Cow::Owned(name.to_lowercase());
    }

    Cow::Borrowed(name)
}

fn encode_name(name: &str) -> Vec<u8> {
    let mut data = Vec::with_capacity(name.len() + 8);

    name.split('.').for_each(|part| {
        let len = part.len() as u8;
        data.push(len);
        data.extend_from_slice(part.as_bytes());
    });
    if !name.ends_with('.') {
        data.push(0);
    }

    data
}

#[cfg(test)]
mod tests {
    use super::*;

    /// returns an absolute domain name which ends with a trailing dot.
    fn absolute_domain(s: &str) -> String {
        if !s.ends_with('.') {
            s.to_string() + "."
        } else {
            s.to_string()
        }
    }

    #[allow(clippy::type_complexity)]
    fn static_hosts() -> Vec<(&'static str, Vec<(&'static str, &'static [&'static str])>)> {
        vec![
            (
                "tests/hosts",
                vec![
                    ("odin", &["127.0.0.2", "127.0.0.3", "::2"]),
                    ("thor", &["127.1.1.1"]),
                    ("ullr", &["127.1.1.2"]),
                    ("ullrhost", &["127.1.1.2"]),
                ],
            ),
            ("tests/singleline-hosts", vec![("odin", &["127.0.0.2"])]),
            (
                "tests/ipv4-hosts",
                vec![
                    ("localhost", &["127.0.0.1", "127.0.0.2", "127.0.0.3"]),
                    ("localhost.localdomain", &["127.0.0.3"]),
                ],
            ),
            (
                "tests/ipv6-hosts",
                vec![
                    (
                        "localhost",
                        &["::1", "fe80::1", "fe80::2%lo0", "fe80::3%lo0"],
                    ),
                    ("localhost.localdomain", &["fe80::3%lo0"]),
                ],
            ),
            (
                "tests/case-hosts",
                vec![
                    ("PreserveMe", &["127.0.0.1", "::1"]),
                    ("PreserveMe.local", &["127.0.0.1", "::1"]),
                ],
            ),
        ]
    }

    #[test]
    fn parse() {
        let input = "::2";

        let addr = input.parse::<IpAddr>().unwrap();
        println!("{}", addr.is_ipv6())
    }

    #[test]
    fn lookup() {
        for (path, entries) in static_hosts() {
            for (domain, ips) in entries {
                let hosts = Hosts::load(path).unwrap();

                let inputs = vec![
                    domain.to_string(),
                    absolute_domain(domain),
                    domain.to_lowercase(),
                    domain.to_uppercase(),
                ];
                for transformed in inputs {
                    let mut addrs = hosts.lookup_ipv4(&transformed).unwrap_or_default();
                    addrs.extend(hosts.lookup_ipv6(&transformed).unwrap_or_default());

                    assert_eq!(
                        addrs
                            .iter()
                            .map(|record| {
                                match record.data {
                                    RecordData::A(addr) => IpAddr::V4(addr),
                                    RecordData::AAAA(addr) => IpAddr::V6(addr),
                                    _ => unreachable!(),
                                }
                            })
                            .collect::<Vec<_>>(),
                        ips.iter()
                            .map(|text| {
                                if let Some((text, _)) = text.split_once('%') {
                                    IpAddr::from_str(text).unwrap()
                                } else {
                                    IpAddr::from_str(text).unwrap()
                                }
                            })
                            .collect::<Vec<_>>(),
                        "lookup {transformed:?} in {path}"
                    );
                }
            }
        }
    }
}
