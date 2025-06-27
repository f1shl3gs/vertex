use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::config::{Config, Hosts};
use crate::proto::{
    Error as DecodeError, HEADER_SIZE, MAX_TTL, Message, Record, RecordClass, RecordData,
    RecordType, decode_message,
};
use crate::singleflight::SingleFlight;

const RESOLVE_FILE_PATH: &str = "/etc/resolv.conf";
const HOSTS_FILE_PATH: &str = "/etc/hosts";

#[derive(Clone, Debug)]
pub enum Error {
    Io(std::io::ErrorKind),

    NoAvailable,

    Decode(DecodeError),
}

#[derive(Debug)]
pub struct Resolver {
    config: Config,
    hosts: Hosts,

    server_offset: AtomicUsize,
    inflight: SingleFlight<(bool, String), Result<Lookup, Error>>,
}

impl Resolver {
    #[inline]
    pub fn new(config: Config, hosts: Hosts) -> Resolver {
        Self {
            config,
            hosts,
            server_offset: Default::default(),
            inflight: SingleFlight::new(),
        }
    }

    pub fn with_defaults() -> std::io::Result<Resolver> {
        let config = Config::load(RESOLVE_FILE_PATH)?;
        let hosts = Hosts::load(HOSTS_FILE_PATH)?;

        Ok(Self {
            config,
            hosts,
            server_offset: Default::default(),
            inflight: SingleFlight::new(),
        })
    }

    pub async fn lookup_ipv4(&self, name: &str) -> Result<Lookup, Error> {
        if let Ok(addr) = name.parse::<Ipv4Addr>() {
            return Ok(Lookup {
                name: Vec::from(name.as_bytes()),
                typ: RecordType::A,
                class: RecordClass::INET,
                records: Arc::from([Record {
                    name: Vec::from(name.as_bytes()),
                    typ: RecordType::A,
                    class: RecordClass::INET,
                    ttl: MAX_TTL,
                    data: RecordData::A(addr),
                }]),
            });
        }

        if let Some(records) = self.hosts.lookup_ipv4(name) {
            return Ok(Lookup {
                name: name.as_bytes().to_vec(),
                typ: RecordType::A,
                class: RecordClass::INET,
                records: Arc::from(records),
            });
        }

        self.inflight
            .call((false, name.to_owned()), async {
                let mut msg = self.lookup(name, RecordType::A, RecordClass::INET).await?;

                let question = msg.questions.remove(0);
                let records = msg
                    .answers
                    .into_iter()
                    .filter_map(|record| {
                        if record.class != RecordClass::INET {
                            return None;
                        }

                        Some(record)
                    })
                    .collect::<Vec<_>>();

                Ok(Lookup {
                    name: question.name,
                    typ: question.typ,
                    class: question.class,
                    records: Arc::from(records),
                })
            })
            .await
    }

    /*
        pub async fn lookup_ipv6(&self, name: &str) -> Result<Lookup, Error> {
            if let Some(records) = self.hosts.lookup_ipv6(name) {
                return Ok(Lookup {
                    name: name.as_bytes().to_vec(),
                    typ: RecordType::AAAA,
                    class: RecordClass::INET,
                    records: Arc::from(records),
                });
            }

            self.inflight
                .call((false, name.to_owned()), async {
                    self.lookup(name, RecordType::AAAA, RecordClass::INET).await
                })
                .await
        }
    */

    /// Lookup records by name, type and class, `/etc/hosts` are ignored
    pub async fn lookup(
        &self,
        name: &str,
        typ: RecordType,
        class: RecordClass,
    ) -> Result<Message, Error> {
        // if (typ == RecordType::A || typ == RecordType::AAAA) && class == RecordClass::INET {}

        let mut buf = [0u8; 512];

        let id = random_id();
        buf[0] = (id >> 8) as u8;
        buf[1] = id as u8;

        // set recursion desired
        let mut flags = 1u16 << 8;
        if self.config.trust_ad {
            flags |= 1u16 << 5;
        }
        buf[2] = (flags >> 8) as u8;
        buf[3] = flags as u8;

        buf[5] = 1; // questions == 1

        let mut pos = HEADER_SIZE;
        name.split('.')
            .map(|name| name.as_bytes())
            .for_each(|name| {
                buf[pos] = name.len() as u8;
                pos += 1;

                if !name.is_empty() {
                    buf[pos..pos + name.len()].copy_from_slice(name);
                    pos += name.len();
                }
            });
        if !name.ends_with('.') {
            buf[pos] = 0;
            pos += 1;
        }

        buf[pos..pos + 2].copy_from_slice(&typ.to_u16().to_be_bytes());
        buf[pos + 2..pos + 4].copy_from_slice(&class.to_u16().to_be_bytes());
        pos += 4;

        // An offset that can be used to determine indices of servers in
        // config.servers when making queries. When the rotate option is
        // enabled, this offset increase. Otherwise, it is always 0.
        let len = self.config.servers.len();
        let offset = if self.config.rotate {
            self.server_offset.fetch_add(1, Ordering::SeqCst)
        } else {
            0
        };

        let mut last_err = None;
        for _ in 0..self.config.attempts {
            for i in 0..len {
                let addr = self.config.servers[(offset + i) % len];

                let result = tokio::time::timeout(self.config.timeout, async {
                    if self.config.use_tcp {
                        self.tcp_round_trip(addr, &buf[..pos], id).await
                    } else {
                        self.udp_round_trip(addr, &buf[..pos], id).await
                    }
                })
                .await;

                match result {
                    Ok(Ok(resp)) => {
                        let msg = if self.config.use_tcp {
                            decode_message(&resp[2..])
                        } else {
                            decode_message(&resp)
                        }
                        .map_err(Error::Decode)?;

                        return Ok(msg);
                    }
                    Ok(Err(err)) => last_err = Some(err),
                    Err(_err) => {
                        // timeout
                        continue;
                    }
                }
            }
        }

        if let Some(err) = last_err {
            return Err(Error::Io(err.kind()));
        }

        Err(Error::NoAvailable)
    }

    async fn udp_round_trip(
        &self,
        addr: SocketAddr,
        req: &[u8],
        id: u16,
    ) -> std::io::Result<Vec<u8>> {
        use tokio::net::UdpSocket;

        let conn = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0)).await?;

        conn.send_to(req, addr).await?;

        // Maximum DNS packet size.
        // Value taken from https://dnsflagday.net/2020/.
        let mut buf = vec![0u8; 1232];
        loop {
            let size = conn.recv(&mut buf).await?;
            buf.truncate(size);

            let got = ((buf[0] as u16) << 8) | buf[1] as u16;
            let flags = ((buf[2] as u16) << 8) | buf[3] as u16;
            if got == id && flags & (1 << 15) != 0 {
                return Ok(buf);
            }
        }
    }

    async fn tcp_round_trip(
        &self,
        addr: SocketAddr,
        req: &[u8],
        id: u16,
    ) -> std::io::Result<Vec<u8>> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let mut stream = TcpStream::connect(addr).await?;

        stream.set_nodelay(true)?;

        stream.write_all(&(req.len() as u16).to_be_bytes()).await?;
        stream.write_all(req).await?;

        // 1280 is a reasonable initial size for IP over Ethernet, see RFC 4035
        let mut buf = vec![0u8; 1280];
        let mut size = 0;
        loop {
            let count = stream.read(&mut buf[size..]).await?;
            size += count;

            if size <= 2 {
                continue;
            }

            let length = ((buf[0] as u16) << 8) | buf[1] as u16;
            if length as usize + 2 == size {
                buf.truncate(size);
                break;
            }
        }

        let got = ((buf[2] as u16) << 8) | buf[3] as u16;
        let flags = ((buf[4] as u16) << 8) | buf[5] as u16;
        if got == id && flags & (1 << 15) != 0 {
            return Ok(buf);
        }

        Err(std::io::Error::other("invalid tcp response"))
    }
}

fn random_id() -> u16 {
    let mut buf = [0u8; 2];
    let ret = unsafe { libc::getrandom(buf.as_mut_ptr().cast(), 2, 0) };
    if ret == -1 {
        panic!("getrandom failed");
    }

    // the endian does not matter here
    u16::from_be_bytes(buf)
}

/// Result of a DNS query when querying for any record type
#[derive(Clone, Debug)]
pub struct Lookup {
    pub name: Vec<u8>,
    pub typ: RecordType,
    pub class: RecordClass,

    pub records: Arc<[Record]>,
}

/// Borrowed view of set of `Records` returned from a `Lookup`
///
/// This is not a zero overhead `Iterator`, because it clones each `Record`
pub struct LookupIntoIter {
    index: usize,
    records: Arc<[Record]>,
}

impl Iterator for LookupIntoIter {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        let record = self.records.get(self.index);
        self.index += 1;
        record.cloned()
    }
}

impl IntoIterator for Lookup {
    type Item = Record;
    type IntoIter = LookupIntoIter;

    /// This is not a free conversion, because the `Records` are cloned.
    fn into_iter(self) -> Self::IntoIter {
        LookupIntoIter {
            index: 0,
            records: Arc::clone(&self.records),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lookup_localhost() {
        let resolver = Resolver::with_defaults().unwrap();

        let msg = resolver
            .lookup("127.0.0.1", RecordType::A, RecordClass::INET)
            .await
            .unwrap();
        println!("{msg:#?}");
    }
}
