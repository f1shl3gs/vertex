use std::collections::HashMap;

use crate::sinks::elasticsearch::request_builder::ElasticsearchRequestBuilder;
use framework::config::UriSerde;
use framework::http::{Auth, MaybeAuth};
use framework::sink::util::service::RequestConfig;
use framework::tls::TlsSettings;
use http::Uri;

use super::config::{ElasticsearchAuth, ElasticsearchConfig};
use super::ElasticsearchCommonMode;
use super::ParseError;

#[derive(Debug)]
pub struct ElasticsearchCommon {
    pub base_url: String,
    pub bulk_uri: Uri,
    pub http_auth: Option<Auth>,
    pub mode: ElasticsearchCommonMode,
    pub request_builder: ElasticsearchRequestBuilder,
    pub tls_settings: TlsSettings,
    pub request: RequestConfig,
    pub query_params: HashMap<String, String>,
}

impl ElasticsearchCommon {
    pub async fn parse_config(config: &ElasticsearchConfig) -> crate::Result<Self> {
        // Test the configured host, but ignore the result
        let uri = format!("{}/_test", &config.endpoint);
        let uri = uri.parse::<Uri>().map_err(|err| ParseError::InvalidHost {
            host: config.endpoint.clone(),
            err,
        })?;
        if uri.host().is_none() {
            return Err(ParseError::HostMustIncludeHostname(config.endpoint.clone()).into());
        }

        let auth = match &config.auth {
            Some(ElasticsearchAuth::Basic { user, password }) => Some(Auth::Basic {
                user: user.clone(),
                password: password.clone(),
            }),
            _ => None,
        };
        let uri = config.endpoint.parse::<UriSerde>()?;
        let http_auth = auth.choose_one(&uri.auth)?;
        let base_url = uri.uri.to_string().trim_end_matches('/').to_owned();

        let mode = config.common_mode()?;
        let doc_type = config.doc_type.clone().unwrap_or_else(|| "_doc".into());
        let request_builder = ElasticsearchRequestBuilder {
            compression: config.compression,
            encoder: ElasticsearchEncoder {
                doc_type,
                suppress_type_name: config.suppress_type_name,
            },
        };
    }
}
