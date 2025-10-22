use codecs::encoding::Transformer;
use configurable::configurable_component;
use framework::batch::{BatchConfig, RealtimeSizeBasedDefaultBatchSettings};
use framework::config::{InputType, SinkConfig, SinkContext};
use framework::http::{Auth, HttpClient};
use framework::sink::http::BatchedHttpSink;
use framework::sink::service::RequestConfig;
use framework::sink::{Buffer, Compression};
use framework::tls::TlsConfig;
use framework::{Healthcheck, Sink};
use futures::{FutureExt, SinkExt};
use url::Url;

use super::sink::{ClickhouseRetryLogic, ClickhouseSink, healthcheck};

fn default_database() -> String {
    "default".to_string()
}

#[configurable_component(sink, name = "clickhouse")]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The endpoint of the ClickHouse server.
    #[configurable(format = "uri", example = "http://localhost:8123")]
    pub endpoint: Url,

    /// The table that data will be inserted into.
    pub table: String,

    /// The database that contains the table that data will be inserted into.
    #[serde(default = "default_database")]
    pub database: String,

    /// Sets `input_format_skip_unknown_fields`, allowing ClickHouse to discard
    /// fields not present in the table schema.
    pub skip_unknown_fields: bool,

    /// Sets `date_time_input_format` to `best_effort`, allowing ClickHouse to
    /// properly parse RFC3339/ISO 8601.
    pub date_time_best_effort: bool,

    /// Compression for HTTP requests.
    #[serde(default = "Compression::gzip_default")]
    pub compression: Compression,

    pub encoding: Transformer,

    pub batch: BatchConfig<RealtimeSizeBasedDefaultBatchSettings>,

    pub auth: Option<Auth>,

    pub request: RequestConfig,

    pub tls: Option<TlsConfig>,

    pub acknowledgements: bool,
}

#[async_trait::async_trait]
#[typetag::serde(name = "clickhouse")]
impl SinkConfig for Config {
    async fn build(&self, cx: SinkContext) -> framework::Result<(Sink, Healthcheck)> {
        let batch = self.batch.into_batch_settings()?;
        let request = self.request.settings();
        let client = HttpClient::new(self.tls.as_ref(), &cx.proxy)?;

        let sink = BatchedHttpSink::with_logic(
            ClickhouseSink::new(self)?,
            Buffer::new(batch.size, self.compression),
            ClickhouseRetryLogic::default(),
            request,
            batch.timeout,
            client.clone(),
        )
        .sink_map_err(|err| {
            error!(message = "Fatal clickhouse sink error", %err);
        });

        let healthcheck = healthcheck(client, self.endpoint.to_string(), self.auth.clone()).boxed();

        Ok((Sink::from_event_sink(sink), healthcheck))
    }

    fn input_type(&self) -> InputType {
        InputType::log()
    }

    fn acknowledgements(&self) -> bool {
        self.acknowledgements
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
