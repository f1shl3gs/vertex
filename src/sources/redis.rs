use futures::{SinkExt, StreamExt};
use redis::InfoDict;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::IntervalStream;
use event::{Event, Metric};
use crate::config::{DataType, SourceConfig, SourceContext, deserialize_duration, serialize_duration, default_interval};
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;
use crate::sources::Source;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RedisSourceConfig {
    // something looks like this, e.g. redis://host:port/db
    url: String,

    #[serde(default = "default_interval")]
    interval: chrono::Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "redis_info")]
impl SourceConfig for RedisSourceConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let cli = &redis::Client::open(&self.url)?;

        Ok(Box::pin(async move {
            let mut output = ctx.out;
            let ticker = IntervalStream::new(tokio::time::interval(self.interval.to_std().unwrap()))
                .take_until(ctx.shutdown);

            while let Some(_) = ticker.next().await {
                match scrap(cli).await {
                    Ok(info) => {

                    }
                    Err(_) => {
                        output.send(Metric::gauge(
                            "redis_up",
                            "redis status",
                            0
                        ).into())
                    }
                }
            }

            Ok(())
        }))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "redis_info"
    }
}

async fn scrap(cli: &redis::Client) -> Result<redis::InfoDict, std::io::Error> {
    let mut conn = cli.get_tokio_connection().await?;
    redis::cmd("INFO").query_async(&mut conn).await?
}

fn convert(infos: redis::InfoDict) -> Vec<Event> {
    todo!()
}

#[cfg(test)]
mod tests {
    use testcontainers::{
        Docker,
        images::redis::Redis
    };
    use redis::Client;
    use super::*;

    #[tokio::test]
    async fn dump_info() {
        let docker = testcontainers::clients::Cli::default();
        let service = docker.run(Redis::default());
        let host_port = service.get_host_port(6379).unwrap();
        let url = format!("redis://localhost:{}", host_port);

        let cli = Client::open(url.as_ref()).unwrap();
        let mut conn = cli.get_tokio_connection().await.unwrap();

        let infos = scrap(&cli).await.unwrap();
        println!("{:?}", infos);
    }
}