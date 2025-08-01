use std::collections::HashMap;

use framework::HealthcheckError;
use framework::config::UriSerde;
use framework::http::{Auth, HttpClient, MaybeAuth};
use framework::sink::util::service::RequestConfig;
use framework::tls::TlsConfig;
use http::{Method, Request, StatusCode, Uri};
use http_body_util::{BodyExt, Full};

use super::ElasticsearchCommonMode;
use super::ParseError;
use super::config::{Config, ElasticsearchAuth};
use super::encoder::ElasticsearchEncoder;
use super::request_builder::ElasticsearchRequestBuilder;

#[derive(Clone, Debug)]
pub struct ElasticsearchCommon {
    pub base_url: String,
    pub bulk_uri: Uri,
    pub http_auth: Option<Auth>,
    pub mode: ElasticsearchCommonMode,
    pub request_builder: ElasticsearchRequestBuilder,
    pub tls: Option<TlsConfig>,
    pub request: RequestConfig,
    pub query_params: HashMap<String, String>,
}

impl ElasticsearchCommon {
    pub async fn parse_config(config: &Config) -> crate::Result<Self> {
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
                password: password.to_string().into(),
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
                transformer: config.encoding.clone(),
                doc_type,
                suppress_type_name: config.suppress_type_name,
            },
        };

        let request_settings = config.request.into_settings();
        let mut query_params = config.query.clone().unwrap_or_default();
        query_params.insert(
            "timeout".into(),
            format!("{}s", request_settings.timeout.as_secs()),
        );

        if let Some(pipeline) = &config.pipeline {
            query_params.insert("pipeline".into(), pipeline.into());
        }

        let mut query = url::form_urlencoded::Serializer::new(String::new());
        for (p, v) in &query_params {
            query.append_pair(&p[..], &v[..]);
        }

        let bulk_url = format!("{}/_bulk?{}", base_url, query.finish());
        let bulk_uri = bulk_url.parse::<Uri>()?;

        let request = config.request.clone();

        Ok(Self {
            base_url,
            bulk_uri,
            http_auth,
            mode,
            request_builder,
            tls: config.tls.clone(),
            request,
            query_params,
        })
    }
}

pub async fn healthcheck(client: HttpClient, uri: String, auth: Option<Auth>) -> crate::Result<()> {
    let mut req = Request::builder()
        .method(Method::GET)
        .uri(format!("{uri}/_cluster/health"))
        .body(Full::default())?;
    if let Some(auth) = auth {
        auth.apply(&mut req);
    }

    let resp = client.send(req).await?;
    let (parts, incoming) = resp.into_parts();

    if parts.status != StatusCode::OK {
        let data = incoming.collect().await?.to_bytes();

        return Err(HealthcheckError::UnexpectedStatus(
            parts.status,
            String::from_utf8_lossy(&data).to_string(),
        )
        .into());
    }

    Ok(())
}
