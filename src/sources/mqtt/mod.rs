mod broker;

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use configurable::configurable_component;
use framework::config::{Output, Resource, SourceConfig, SourceContext};
use framework::tcp::TcpKeepaliveConfig;
use framework::tls::{MaybeTlsListener, TlsConfig};
use framework::{Pipeline, ShutdownSignal, Source};

const fn default_listen() -> SocketAddr {
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1883))
}

/// The MQTT source allows to retrieve messages/data from MQTT control packets
/// over a TCP connection. The incoming data to receive must be a JSON map.
#[configurable_component(source, name = "mqtt")]
struct Config {
    #[serde(default = "default_listen")]
    listen: SocketAddr,

    tls: Option<TlsConfig>,

    keepalive: Option<TcpKeepaliveConfig>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "mqtt")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let output = cx.output;
        let shutdown = cx.shutdown;
        let listener = MaybeTlsListener::bind(&self.listen, self.tls.as_ref()).await?;

        Ok(Box::pin(run(listener, self.keepalive, output, shutdown)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.listen)]
    }

    /// TODO: it could be true, `PUBACK`
    fn can_acknowledge(&self) -> bool {
        false
    }
}

async fn run(
    mut listener: MaybeTlsListener,
    keepalive: Option<TcpKeepaliveConfig>,
    output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown => break,
            res = listener.accept() => {
                match res {
                    Ok(mut stream) => {
                        let output = output.clone();
                        let remote_addr = stream.peer_addr();

                        if let Some(keepalive) = &keepalive {
                            if let Err(err) = stream.set_keepalive(*keepalive) {
                                error!(
                                    message = "set keepalive failed",
                                    ?err
                                );

                                continue;
                            }
                        }

                        tokio::spawn(broker::serve_connection(remote_addr, stream, output));
                    },
                    Err(err) => {
                        error!(
                            message = "accept tcp connection failed",
                            ?err
                        );

                        continue;
                    }
                };
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
