//! Read system DNS config from /etc/resolv.conf

use std::io::{BufRead, BufReader};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use super::{Config, default_nameservers, default_search, ensure_rooted};

impl Default for Config {
    fn default() -> Self {
        Config {
            servers: default_nameservers(),
            search: default_search(),
            ndots: 1,
            timeout: Duration::from_secs(5),
            attempts: 2,

            rotate: false,
            unknown_opt: false,
            #[cfg(target_os = "openbsd")]
            lookup: vec![],
            mtime: UNIX_EPOCH,
            single_request: false,
            use_tcp: false,
            trust_ad: false,
            no_reload: false,
        }
    }
}

impl Config {
    // See resolv.conf(5) on a Linux machine
    pub fn load(path: impl AsRef<Path>) -> std::io::Result<Config> {
        let file = std::fs::File::open(path)?;
        let mtime = file.metadata()?.modified()?;

        let mut config = Config {
            servers: vec![],
            search: vec![],
            ndots: 1,
            timeout: Duration::from_secs(5),
            attempts: 2,
            mtime,
            ..Default::default()
        };

        let mut lines = BufReader::new(file).lines();
        while let Some(Ok(line)) = lines.next() {
            let Some(mut fields) = line
                .split([';', '#'])
                .next()
                .map(|line| line.split_ascii_whitespace())
            else {
                continue;
            };

            let Some(keyword) = fields.next() else {
                // empty or comment line
                continue;
            };

            match keyword {
                // add one name server
                "nameserver" => {
                    // small, but the standard limit
                    if let Some(field) = fields.next() {
                        // One more check: make sure server name is just an IP address
                        // Otherwise we need DNS to look it up
                        let Ok(ip) = field.parse::<IpAddr>() else {
                            continue;
                        };

                        config.servers.push(SocketAddr::new(ip, 53));
                    }
                }

                // set search path to just this domain
                "domain" => {
                    if let Some(search) = fields.next() {
                        config.search = vec![ensure_rooted(search)]
                    }
                }

                // set search path to given servers
                "search" => {
                    config.search = fields
                        .filter_map(|field| {
                            if field == "." || field.is_empty() {
                                None
                            } else {
                                Some(ensure_rooted(field))
                            }
                        })
                        .collect::<Vec<_>>();
                }

                // magic options
                "options" => {
                    for field in fields {
                        if let Some(stripped) = field.strip_prefix("ndots:") {
                            let num = stripped.parse::<i32>().unwrap_or(0);
                            config.ndots = num.clamp(0, 15);
                        } else if let Some(stripped) = field.strip_prefix("timeout:") {
                            let mut num = stripped.parse::<i64>().unwrap_or(0);
                            if num < 1 {
                                num = 1;
                            }

                            config.timeout = Duration::from_secs(num as u64);
                        } else if let Some(stripped) = field.strip_prefix("attempts:") {
                            let mut num = stripped.parse::<u32>().unwrap_or(0);
                            if num < 1 {
                                num = 1;
                            }

                            config.attempts = num;
                        } else if field == "rotate" {
                            config.rotate = true;
                        } else if field == "single-request" || field == "single-request-reopen" {
                            // Linux option:
                            // http://man7.org/linux/man-pages/man5/resolv.conf.5.html
                            // "By default, glibc performs IPv4 and IPv6 lookups in parallel [...]
                            //  This option disables the behavior and makes glibc
                            //  perform the IPv6 and IPv4 requests sequentially."
                            config.single_request = true;
                        } else if field == "use-vc" || field == "usevc" || field == "tcp" {
                            // Linux (use-vc), FreeBSD (usevc) and OpenBSD (tcp) option:
                            // http://man7.org/linux/man-pages/man5/resolv.conf.5.html
                            // "Sets RES_USEVC in _res.options.
                            //  This option forces the use of TCP for DNS resolutions."
                            // https://www.freebsd.org/cgi/man.cgi?query=resolv.conf&sektion=5&manpath=freebsd-release-ports
                            // https://man.openbsd.org/resolv.conf.5
                            config.use_tcp = true;
                        } else if field == "trust-ad" {
                            config.trust_ad = true;
                        } else if field == "edns0" {
                            // We use EDNS by default, ignore this option
                        } else if field == "no-reload" {
                            config.no_reload = true
                        } else {
                            config.unknown_opt = true
                        }
                    }
                }

                #[cfg(target_os = "openbsd")]
                "lookup" => {
                    // OpenBSD option:
                    // https://www.openbsd.org/cgi-bin/man.cgi/OpenBSD-current/man5/resolv.conf.5
                    // "the legal space-separated values are: bind, file, yp"
                    config.lookup = fields.map(ToString::to_string).collect();
                }

                _ => {
                    config.unknown_opt = true;
                }
            }
        }

        if config.servers.is_empty() {
            config.servers = default_nameservers();
        }

        if config.search.is_empty() {
            config.search = default_search();
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom, Write};

    use crate::config::default_search_with;

    fn read_config_tests() -> Vec<(&'static str, Config)> {
        vec![
            (
                "tests/resolv.conf",
                Config {
                    servers: vec![
                        "8.8.8.8:53".parse().unwrap(),
                        "[2001:4860:4860::8888]:53".parse().unwrap(),
                    ],
                    search: vec!["localdomain.".into()],
                    ndots: 5,
                    timeout: Duration::from_secs(10),
                    attempts: 3,
                    rotate: true,
                    unknown_opt: true,
                    ..Default::default()
                },
            ),
            (
                "tests/domain-resolv.conf",
                Config {
                    servers: vec!["8.8.8.8:53".parse().unwrap()],
                    search: vec!["localdomain.".into()],
                    ndots: 1,
                    timeout: Duration::from_secs(5),
                    attempts: 2,
                    ..Default::default()
                },
            ),
            (
                "tests/search-resolv.conf",
                Config {
                    servers: vec!["8.8.8.8:53".parse().unwrap()],
                    search: vec!["test.".into(), "invalid.".into()],
                    ndots: 1,
                    timeout: Duration::from_secs(5),
                    attempts: 2,
                    ..Default::default()
                },
            ),
            (
                "tests/search-single-dot-resolv.conf",
                Config {
                    servers: vec!["8.8.8.8:53".parse().unwrap()],
                    search: vec![],
                    ndots: 1,
                    timeout: Duration::from_secs(5),
                    attempts: 2,
                    ..Default::default()
                },
            ),
            (
                "tests/empty-resolv.conf",
                Config {
                    servers: default_nameservers(),
                    search: vec!["domain.local.".into()],
                    ndots: 1,
                    timeout: Duration::from_secs(5),
                    attempts: 2,
                    ..Default::default()
                },
            ),
            (
                "tests/invalid-ndots-resolv.conf",
                Config {
                    servers: default_nameservers(),
                    ndots: 0,
                    timeout: Duration::from_secs(5),
                    attempts: 2,
                    search: vec!["domain.local.".into()],
                    ..Default::default()
                },
            ),
            (
                "tests/large-ndots-resolv.conf",
                Config {
                    servers: default_nameservers(),
                    ndots: 15,
                    timeout: Duration::from_secs(5),
                    attempts: 2,
                    search: vec!["domain.local.".into()],
                    ..Default::default()
                },
            ),
            (
                "tests/negative-ndots-resolv.conf",
                Config {
                    servers: default_nameservers(),
                    ndots: 0,
                    timeout: Duration::from_secs(5),
                    attempts: 2,
                    search: vec!["domain.local.".into()],
                    ..Default::default()
                },
            ),
            (
                "tests/openbsd-resolv.conf",
                Config {
                    servers: vec![
                        "169.254.169.254:53".parse().unwrap(),
                        "10.240.0.1:53".parse().unwrap(),
                    ],
                    search: vec!["c.symbolic-datum-552.internal.".into()],
                    #[cfg(target_os = "openbsd")]
                    lookup: vec!["file", "bind"],
                    ndots: 1,
                    timeout: Duration::from_secs(5),
                    attempts: 2,
                    ..Default::default()
                },
            ),
            (
                "tests/single-request-resolv.conf",
                Config {
                    servers: default_nameservers(),
                    search: vec!["domain.local.".into()],
                    ndots: 1,
                    single_request: true,
                    timeout: Duration::from_secs(5),
                    attempts: 2,
                    ..Default::default()
                },
            ),
            (
                "tests/single-request-reopen-resolv.conf",
                Config {
                    servers: default_nameservers(),
                    search: vec!["domain.local.".into()],
                    ndots: 1,
                    single_request: true,
                    timeout: Duration::from_secs(5),
                    attempts: 2,
                    ..Default::default()
                },
            ),
            (
                "tests/linux-use-vc-resolv.conf",
                Config {
                    servers: default_nameservers(),
                    search: vec!["domain.local.".into()],
                    ndots: 1,
                    use_tcp: true,
                    timeout: Duration::from_secs(5),
                    attempts: 2,
                    ..Default::default()
                },
            ),
            (
                "tests/freebsd-usevc-resolv.conf",
                Config {
                    servers: default_nameservers(),
                    search: vec!["domain.local.".into()],
                    ndots: 1,
                    use_tcp: true,
                    timeout: Duration::from_secs(5),
                    attempts: 2,
                    ..Default::default()
                },
            ),
            (
                "tests/openbsd-tcp-resolv.conf",
                Config {
                    servers: default_nameservers(),
                    search: vec!["domain.local.".into()],
                    ndots: 1,
                    use_tcp: true,
                    timeout: Duration::from_secs(5),
                    attempts: 2,
                    ..Default::default()
                },
            ),
        ]
    }

    #[test]
    fn load_config() {
        for (path, mut expected) in read_config_tests() {
            // correct mtime
            expected.mtime = std::fs::metadata(path).unwrap().modified().unwrap();

            if expected.search.is_empty() {
                expected.search = default_search_with(b"host.domain.local");
            }

            let got = Config::load(path).unwrap();

            assert_eq!(got.servers, expected.servers, "{path}");
            assert_eq!(got.search, expected.search, "{path}");
            assert_eq!(got.ndots, expected.ndots, "{path}");
            assert_eq!(got.timeout, expected.timeout, "{path}");
            assert_eq!(got.attempts, expected.attempts, "{path}");
            assert_eq!(got.rotate, expected.rotate, "{path}");
            #[cfg(target_os = "openbsd")]
            assert_eq!(got.unknown_opt, expected.unknown_opt, "{}", path);
            assert_eq!(got.mtime, expected.mtime, "{path}");
            assert_eq!(got.single_request, expected.single_request, "{path}");
            assert_eq!(got.use_tcp, expected.use_tcp, "{path}");
            assert_eq!(got.trust_ad, expected.trust_ad, "{path}");
            assert_eq!(got.no_reload, expected.no_reload, "{path}");
        }
    }

    #[test]
    fn none_exist_file() {
        let config = Config::load("a-non-existent-file").unwrap_or_default();
        assert_eq!(config.servers, default_nameservers());
    }

    #[test]
    fn dns_default_search() {
        for (name, want, _err) in [
            ("host.long.domain.local", vec!["long.domain.local."], false),
            ("host.local", vec!["local."], false),
            ("host", vec![], false),
            // ("host.domain.local", Vec::<&str>::new(), true),
            ("foo.", Vec::<&str>::new(), false),
        ] {
            let got = default_search_with(name.as_bytes());
            assert_eq!(got, want, "input {name}");
        }
    }

    #[test]
    fn ndots() {
        let path = std::env::temp_dir().join("ndots.txt");

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .truncate(true)
            .open(&path)
            .unwrap();

        for (content, ndots) in [
            ("options ndots:0", 0),
            ("options ndots:1", 1),
            ("options ndots:15", 15),
            ("options ndots:16", 15),
            ("options ndots:-1", 0),
            ("", 1),
        ] {
            file.set_len(0).unwrap();
            file.seek(SeekFrom::Start(0)).unwrap();
            file.write_all(content.as_bytes()).unwrap();
            file.flush().unwrap();

            let config = Config::load(&path).unwrap();
            assert_eq!(config.ndots, ndots, "content: *{content}*");
        }

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn attempts() {
        let path = std::env::temp_dir().join("attempts.txt");

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .truncate(true)
            .open(&path)
            .unwrap();

        for (content, attempts) in [
            ("options attempts:0", 1),
            ("options attempts:1", 1),
            ("options attempts:15", 15),
            ("options attempts:16", 16),
            ("options attempts:-1", 1),
            ("options attempt:", 2),
        ] {
            file.set_len(0).unwrap();
            file.seek(SeekFrom::Start(0)).unwrap();
            file.write_all(content.as_bytes()).unwrap();
            file.flush().unwrap();

            let config = Config::load(&path).unwrap();
            assert_eq!(config.attempts, attempts, "content: *{content}*");
        }
    }
}
