use std::time::Duration;

use async_trait::async_trait;
use chrono::{Local, SecondsFormat};
use codecs::DecodingConfig;
use codecs::decoding::{DeserializerConfig, FramingConfig, StreamDecodingError};
use configurable::{Configurable, configurable_component};
use framework::Source;
use framework::config::{Output, SourceConfig, SourceContext};
use futures::StreamExt;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio_util::codec::FramedRead;

use super::{default_decoding, default_framing_message_based};

static TLDS: [&str; 4] = ["com", "cn", "net", "org"];

static DOMAINS: [&str; 8] = [
    "some", "random", "names", "we", "make", "up", "for", "testing",
];

static NAMES: [&str; 8] = [
    "anna",
    "bella",
    "catherine",
    "diana",
    "elsa",
    "flora",
    "grace",
    "hilda",
];

static HTTP_METHODS: [&str; 7] = ["DELETE", "GET", "HEAD", "OPTION", "PATCH", "POST", "PUT"];
static HTTP_VERSIONS: [&str; 3] = ["HTTP/1.0", "HTTP/1.1", "HTTP/2.0"];
static HTTP_CODES: [usize; 15] = [
    200, 300, 301, 302, 304, 307, 400, 401, 403, 404, 410, 500, 501, 503, 550,
];
static HTTP_ENDPOINTS: [&str; 9] = [
    "/wp-admin",
    "/controller/setup",
    "/user/booperbot124",
    "/apps/deploy",
    "/observability/metrics/production",
    "/secret-info/open-sesame",
    "/booper/bopper/mooper/mopper",
    "/do-not-access/needs-work",
    "/this/endpoint/prints/money",
];

static ERROR_MESSAGES: [&str; 9] = [
    "There's a breach in the warp core, captain",
    "Great Scott! We're never gonna reach 88 mph with the flux capacitor in its current state!",
    "You're not gonna believe what just happened",
    "#hugops to everyone who has to deal with this",
    "Take a breath, let it go, walk away",
    "A bug was encountered but not in Vector, which doesn't have bugs",
    "We're gonna need a bigger boat",
    "Maybe we just shouldn't use computers",
    "Pretty pretty pretty good",
];

static USER_AGENTS: [&str; 50] = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36 Edg/98.0.1108.62",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0.3 Safari/605.1.15",
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:97.0) Gecko/20100101 Firefox/97.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0.3 Safari/605.1.15",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:97.0) Gecko/20100101 Firefox/97.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0.3 Safari/605.1.15",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:97.0) Gecko/20100101 Firefox/97.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36 Edg/98.0.1108.62",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:97.0) Gecko/20100101 Firefox/97.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0.3 Safari/605.1.15",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36",
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:97.0) Gecko/20100101 Firefox/97.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36 Edg/97.0.1072.71",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36 Edg/97.0.1072.71",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36 Edg/97.0.1072.71",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36 Edg/97.0.1072.71",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36 Edg/97.0.1072.71",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36 Edg/98.0.1108.62",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36 Edg/98.0.1108.62",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36 Edg/98.0.1108.62",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:97.0) Gecko/20100101 Firefox/97.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36 Edg/97.0.1072.71",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:97.0) Gecko/20100101 Firefox/97.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36",
];

fn random_from_slice(s: &'static [&'static str]) -> &'static str {
    s[rand::rng().random_range(0..s.len())]
}

fn random_number(min: usize, max: usize) -> usize {
    rand::rng().random_range(min..max)
}

fn random_domain() -> String {
    let name = random_from_slice(&DOMAINS);
    let tld = random_from_slice(&TLDS);
    format!("{name}.{tld}")
}

fn random_ipv4() -> String {
    let mut rnd = rand::rng();

    loop {
        let a = rnd.random_range(1..255);

        // 1.0.0.0 – 9.255.255.255
        if a > 0 && a < 10 {
            let b = rnd.random_range(0..255);
            let c = rnd.random_range(0..255);
            let d = rnd.random_range(0..255);

            return format!("{a}.{b}.{c}.{d}");
        }

        // 11.0.0.0 – 126.255.255.255
        if (11..=126).contains(&a) {
            let b = rnd.random_range(0..255);
            let c = rnd.random_range(0..255);
            let d = rnd.random_range(0..255);

            return format!("{a}.{b}.{c}.{d}");
        }

        // 129.0.0.0 – 169.253.255.255
        if (129..=169).contains(&a) {
            let b = if a == 169 {
                rnd.random_range(1..253)
            } else {
                rnd.random_range(1..255)
            };

            let c = rnd.random_range(0..255);
            let d = rnd.random_range(0..255);

            return format!("{a}.{b}.{c}.{d}");
        }

        // 169.255.0.0 – 172.15.255.255
        if (169..=172).contains(&a) {
            let b = if a == 172 {
                rnd.random_range(1..15)
            } else if a == 169 {
                255
            } else {
                rnd.random_range(1..255)
            };

            let c = rnd.random_range(0..255);
            let d = rnd.random_range(0..255);

            return format!("{a}.{b}.{c}.{d}");
        }

        // 172.32.0.0 – 191.0.1.255
        if (172..=191).contains(&a) {
            let b = if a == 172 {
                rnd.random_range(32..255)
            } else if a == 191 {
                0
            } else {
                rnd.random_range(1..255)
            };

            let c = rnd.random_range(0..255);
            let d = rnd.random_range(0..255);

            return format!("{a}.{b}.{c}.{d}");
        }

        // 192.0.3.0 – 192.88.98.255
        // 192.88.100.0 – 192.167.255.255
        // 192.169.0.0 – 198.17.255.255
        if a == 192 {
            let b = rnd.random_range(0..88);

            // 192.0.3.0 – 192.88.98.255
            if b < 88 {
                let c = if b == 88 {
                    rnd.random_range(0..98)
                } else {
                    rnd.random_range(0..255)
                };

                let d = rnd.random_range(0..255);

                return format!("{a}.{b}.{c}.{d}");
            }

            // 192.88.100.0 – 192.167.255.255
            if (88..167).contains(&b) {
                let c = if b == 88 {
                    rnd.random_range(100..255)
                } else {
                    rnd.random_range(0..255)
                };

                let d = rnd.random_range(0..255);

                return format!("{a}.{b}.{c}.{d}");
            }

            if b == 169 {
                let c = rnd.random_range(0..255);
                let d = rnd.random_range(0..255);

                return format!("{a}.{b}.{c}.{d}");
            }

            continue;
        }

        // part of 192.169.0.0 – 198.17.255.255
        if a > 192 && a <= 198 {
            let b = if a == 198 {
                rnd.random_range(0..17)
            } else {
                rnd.random_range(0..255)
            };

            let c = rnd.random_range(0..255);
            let d = rnd.random_range(0..255);

            return format!("{a}.{b}.{c}.{d}");
        }

        // 198.20.0.0 – 223.255.255.255
        if (198..=223).contains(&a) {
            let b = if a == 198 {
                rnd.random_range(20..255)
            } else {
                rnd.random_range(0..255)
            };

            let c = rnd.random_range(0..255);
            let d = rnd.random_range(0..255);

            return format!("{a}.{b}.{c}.{d}");
        }
    }
}

fn internal_ipv4() -> String {
    let mut rnd = rand::rng();

    let b = rnd.random_range(1..5);
    let c = rnd.random_range(1..255);
    let d = rnd.random_range(1..255);

    format!("10.{b}.{c}.{d}")
}

fn syslog_5424_log_line() -> String {
    // RFC5424
    // <65>2 2020-11-05T18:11:43.975Z chiefubiquitous.io totam 6899 ID44 - Something bad happened

    format!(
        "<{}>{} {} {} {} {} ID{} - {}",
        random_number(0, 191),
        random_number(0, 3),
        Local::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        random_domain(),
        random_from_slice(&NAMES),
        random_number(100, 9999),
        random_number(1, 999),
        random_from_slice(&ERROR_MESSAGES),
    )
}

fn json_log_line() -> String {
    let referer = format!(
        "https://{}{}",
        random_domain(),
        random_from_slice(&HTTP_ENDPOINTS)
    );

    format!(
        "{{\"remote_addr\":\"{}\",\"datetime\":\"{}\",\"method\":\"{}\",\"request_uri\":\"{}\",\"protocol\":\"{}\",\"status\":{},\"bytes_sent\":{},\"bytes_received\":{},\"http_referer\":\"{}\",\"ua\":\"{}\",\"upstream_addr\":\"{}\"}}",
        random_ipv4(),
        Local::now().format("%d/%b/%Y:%T"),
        random_from_slice(&HTTP_METHODS),
        random_from_slice(&HTTP_ENDPOINTS),
        random_from_slice(&HTTP_VERSIONS),
        HTTP_CODES[random_number(0, HTTP_CODES.len())],
        random_number(1000, 100000),
        random_number(128, 50000),
        referer,
        random_from_slice(&USER_AGENTS),
        internal_ipv4(),
    )
}

#[derive(Clone, Configurable, Debug, Default, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OutputFormat {
    /// Lines are chosen at random from the list specified using `lines`
    Shuffle {
        /// If `true`, each output line starts with an increasing sequence number,
        /// beginning with 0.
        #[serde(default)]
        sequence: bool,
        /// The list of lines to output
        lines: Vec<String>,
    },

    /// Randomly generated logs in Syslog format.
    Syslog,

    /// Randomly generated HTTP server logs in [JSON][json] format.
    #[default]
    Json,
}

impl OutputFormat {
    fn generate_line(&self, n: usize) -> String {
        match self {
            OutputFormat::Shuffle { sequence, lines } => {
                use rand::prelude::IndexedRandom;

                // unwrap can be called here because `lines` can't be empty.
                let line = lines.choose(&mut rand::rng()).unwrap();

                if *sequence {
                    format!("{n} {line}")
                } else {
                    line.to_string()
                }
            }
            OutputFormat::Syslog => syslog_5424_log_line(),
            OutputFormat::Json => json_log_line(),
        }
    }

    fn validate(&self) -> Result<(), crate::Error> {
        if let OutputFormat::Shuffle { lines, .. } = self
            && lines.is_empty()
        {
            return Err("A non-empty list of lines is required for the shuffle format".into());
        }

        Ok(())
    }
}

const fn default_interval() -> Duration {
    Duration::from_secs(1)
}

const fn default_count() -> usize {
    usize::MAX
}

#[configurable_component(source, name = "generate")]
#[serde(deny_unknown_fields)]
struct Config {
    /// How many logs to produce.
    #[serde(default = "default_count")]
    count: usize,

    /// The amount of time, to pause between each batch of output lines.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    #[serde(default = "default_framing_message_based")]
    framing: FramingConfig,

    #[serde(default = "default_decoding")]
    decoding: DeserializerConfig,

    /// The format of the randomly generated output.
    #[serde(default)]
    format: OutputFormat,
}

#[async_trait]
#[typetag::serde(name = "generate")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        self.format.validate()?;

        let decoder = DecodingConfig::new(self.framing.clone(), self.decoding.clone()).build()?;
        let format = self.format.clone();

        let count = self.count;
        let mut ticker = tokio::time::interval(self.interval);
        let mut output = cx.output;
        let mut shutdown = cx.shutdown;

        Ok(Box::pin(async move {
            for n in 0..count {
                tokio::select! {
                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                let line = format.generate_line(n);
                let mut stream = FramedRead::new(line.as_bytes(), decoder.clone());
                while let Some(next) = stream.next().await {
                    match next {
                        Ok((events, _byte_size)) => {
                            if let Err(err) = output.send(events).await {
                                error!(message = "send demo log to output failed", %err);

                                break;
                            }
                        }
                        Err(err) => {
                            if !err.can_continue() {
                                break;
                            }
                        }
                    }
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
