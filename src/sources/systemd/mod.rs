mod dbus;
mod resolved;
mod service;
mod units;
mod version;
mod virtualization;
mod watchdog;

use std::time::Duration;

use configurable::configurable_component;
use dbus::Client;
use framework::Source;
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval, serde_regex};

fn default_include() -> regex::Regex {
    regex::Regex::new(".+").unwrap()
}

fn default_exclude() -> regex::Regex {
    regex::Regex::new(".+\\.(device)").unwrap()
}

#[configurable_component(source, name = "systemd")]
struct Config {
    /// Regex of systemd units to include. Units must both match include and not match
    /// exclude to be included.
    #[serde(default = "default_include", with = "serde_regex")]
    include: regex::Regex,

    /// Regexp of systemd units to exclude. Units must both match include and not match
    /// exclude to be included.
    #[serde(default = "default_exclude", with = "serde_regex")]
    exclude: regex::Regex,

    /// This sources collects metrics on an interval.
    #[serde(with = "humanize::duration::serde", default = "default_interval")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "systemd")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let interval = self.interval;
        let include = self.include.clone();
        let exclude = self.exclude.clone();

        let mut shutdown = cx.shutdown;
        let mut output = cx.output;

        Ok(Box::pin(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                let mut client = match Client::connect().await {
                    Ok(client) => client,
                    Err(err) => {
                        warn!(
                            message = "Failed to connect to systemd's dbus",
                            %err
                        );
                        continue;
                    }
                };

                // N.B.
                // it takes around 700ms in serial mode, so we don't need to
                // make it parallel.

                let version = match version::collect(&mut client).await {
                    Ok((version, metric)) => {
                        if let Err(_err) = output.send(metric).await {
                            break;
                        }
                        version
                    }
                    Err(err) => {
                        warn!(
                            message = "failed to get systemd version",
                            %err
                        );

                        0.0
                    }
                };

                match virtualization::collect(&mut client).await {
                    Ok(metric) => {
                        if let Err(_err) = output.send(metric).await {
                            break;
                        }
                    }
                    Err(err) => {
                        warn!(
                            message = "failed to get systemd virtualization type",
                            %err
                        );
                    }
                }

                let mut metrics =
                    match units::collect(&mut client, &include, &exclude, version).await {
                        Ok(metrics) => metrics,
                        Err(err) => {
                            warn!(
                                message = "failed to collect systemd units metrics",
                                %err
                            );
                            continue;
                        }
                    };

                match service::collect(&mut client).await {
                    Ok(partial) => metrics.extend(partial),
                    Err(err) => {
                        warn!(
                            message = "failed to collect systemd metrics",
                            %err
                        );
                    }
                }

                match resolved::collect(&mut client).await {
                    Ok(partial) => metrics.extend(partial),
                    Err(err) => {
                        warn!(
                            message = "Failed to collect resolved metrics",
                            %err
                        );
                    }
                }

                match watchdog::collect(&mut client).await {
                    Ok(partial) => metrics.extend(partial),
                    Err(err) => {
                        warn!(
                            message = "failed to collect watchdog metrics",
                            %err
                        );
                    }
                }

                if let Err(_err) = output.send(metrics).await {
                    break;
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
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

/*
#[cfg(test)]
mod dump {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{UnixSocket, UnixStream};

    #[tokio::test]
    async fn unix_dump() {
        let sock = UnixSocket::new_stream().unwrap();

        sock.bind("proxy.sock").unwrap();
        let listener = sock.listen(1024).unwrap();

        let upstream = UnixStream::connect("/var/run/dbus/system_bus_socket")
            .await
            .unwrap();

        let (peer, _addr) = listener.accept().await.unwrap();

        let (mut peer_reader, mut peer_writer) = peer.into_split();
        let (mut upstream_reader, mut upstream_writer) = upstream.into_split();

        tokio::task::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                let size = peer_reader.read(&mut buf).await.unwrap();
                if size == 0 {
                    continue;
                }

                println!("send: {} {:?}", size, &buf[..size]);
                println!("send: {}", String::from_utf8_lossy(&buf[..size]));

                upstream_writer.write_all(&buf[..size]).await.unwrap();
            }
        });

        let mut buf = [0; 1024];
        loop {
            let size = upstream_reader.read(&mut buf).await.unwrap();
            println!("recv: {} {:?}", size, &buf[..size]);
            println!("recv: {}", String::from_utf8_lossy(&buf[..size]));

            peer_writer.write_all(&buf[..size]).await.unwrap();
        }
    }
}
*/
