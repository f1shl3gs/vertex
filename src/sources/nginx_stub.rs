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
                waiting: 106,
            }
        )
    }
}

#[cfg(test)]
mod integration_tests {
    mod nginx {
        use std::collections::HashMap;
        use testcontainers::{Container, Docker, Image, WaitForMessage};
        use testcontainers::images::zookeeper::Zookeeper;

        const CONTAINER_IDENTIFIER: &str = "nginx";
        const DEFAULT_TAG: &str = "1.21.3";

        #[derive(Debug, Default, Clone)]
        pub struct NginxArgs;

        impl IntoIterator for NginxArgs {
            type Item = String;
            type IntoIter = ::std::vec::IntoIter<String>;

            fn into_iter(self) -> Self::IntoIter {
                vec![].into_iter()
            }
        }

        #[derive(Debug)]
        pub struct Nginx {
            tag: String,
            arguments: NginxArgs,
            envs: HashMap<String, String>,
            volumes: HashMap<String, String>,
        }

        impl Default for Nginx {
            fn default() -> Self {
                Self {
                    tag: DEFAULT_TAG.to_string(),
                    arguments: NginxArgs,
                    envs: HashMap::new(),
                    volumes: HashMap::new(),
                }
            }
        }

        impl Image for Nginx {
            type Args = NginxArgs;
            type EnvVars = HashMap<String, String>;
            type Volumes = HashMap<String, String>;
            type EntryPoint = std::convert::Infallible;

            fn descriptor(&self) -> String {
                format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
            }

            fn wait_until_ready<D: Docker>(&self, container: &Container<'_, D, Self>) {
                container
                    .logs()
                    .stdout
                    .wait_for_message("start worker process")
                    .unwrap();
            }

            fn args(&self) -> Self::Args {
                self.arguments.clone()
            }

            fn env_vars(&self) -> Self::EnvVars {
                self.envs.clone()
            }

            fn volumes(&self) -> Self::Volumes {
                self.volumes.clone()
            }

            fn with_args(self, arguments: Self::Args) -> Self {
                Nginx {
                    arguments,
                    ..self
                }
            }
        }

        impl Nginx {
            pub fn with_tag(&mut self, tag_str: &str) -> &Self {
                self.tag = tag_str.to_string();
                self
            }

            pub fn with_volume(&mut self, src: &str, target: &str) -> &Self {
                self.volumes.insert(src.to_string(), target.to_string());
                self
            }
        }
    }

    use std::convert::TryInto;
    use testcontainers::Docker;
    use nginx::Nginx;
    use super::NginxStubStatus;

    #[tokio::test]
    async fn test_fetch_and_convert() {
        let docker = testcontainers::clients::Cli::default();
        let image = Nginx::default()
            .with_volume("testdata/nginx/nginx.conf", "/etc/nginx/conf.d/stub_status.conf")
            .with_volume("testdata/nginx/nginx_auth_basic.conf", "/etc/nginx/nginx_auth_basic.conf");
        let service = docker.run(image);
        let host_port = service.get_host_port(8000).unwrap();

        let cli = hyper::Client::new();
        let uri = format!("http://127.0.0.1:{}/basic_status", host_port);
        let resp = cli.get(uri.into())
            .await
            .unwrap();

        let s = hyper::body::to_bytes(resp)
            .await
            .unwrap();

        let s = s.to_str().unwrap();
        let status = s.try_into().unwrap();
    }
}