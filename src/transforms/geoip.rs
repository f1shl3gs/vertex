use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use configurable::configurable_component;
use event::Events;
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};
use maxminddb;
use serde::Serialize;

#[configurable_component(transform, name = "geoip")]
#[derive(Clone, Debug)]
#[serde(deny_unknown_fields)]
struct Config {
    /// Path to the `MaxMind GeoIP2` or `GeoLite2` binary city database file. Other
    /// databases, such as the country database, are not supported.
    ///
    /// GeoIP2: https://dev.maxmind.com/geoip/geoip2/downloadable
    /// GeoLite2: https://dev.maxmind.com/geoip/geoip2/geolite2/#Download_Access
    #[configurable(required)]
    pub database: PathBuf,

    /// The field to insert the resulting GeoIP data into.
    #[serde(default = "default_target")]
    #[configurable(required)]
    pub target: String,

    /// The field name that contains the IP address. This field should contain a
    /// valid IPv4 or IPv6 address.
    #[configurable(required, example = ".foo.bar[2]")]
    pub source: String,

    /// valid locales are: “de”, "en", “es”, “fr”, “ja”, “pt-BR”, “ru”, and “zh-CN” .
    ///
    /// https://dev.maxmind.com/geoip/docs/databases/city-and-country?lang=en
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_target() -> String {
    "geoip".to_string()
}

fn default_locale() -> String {
    "en".to_string()
}

#[async_trait]
#[typetag::serde(name = "geoip")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let database = maxminddb::Reader::open_readfile(self.database.clone())?;

        Ok(Transform::function(Geoip {
            database: Arc::new(database),
            source: self.source.clone(),
            target: self.target.clone(),
            locale: self.locale.clone(),
        }))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

// MaxMind GeoIP database files have a type field we can use to recognize specific
// products. If we encounter one of these two types, we look for ASN/ISP information;
// otherwise we expect to be working with a City database.
const ASN_TYPE: &str = "GeoLite2-ASN";
const ISP_TYPE: &str = "GeoIP2-ISP";

#[derive(Clone, Debug)]
struct Geoip {
    database: Arc<maxminddb::Reader<Vec<u8>>>,
    source: String,
    target: String,
    locale: String,
}

impl Geoip {
    fn has_isp_db(&self) -> bool {
        self.database.metadata.database_type == ASN_TYPE
            || self.database.metadata.database_type == ISP_TYPE
    }
}

#[derive(Default, Serialize)]
struct Isp<'a> {
    autonomous_system_number: i64,
    autonomous_system_organization: &'a str,
    isp: &'a str,
    organization: &'a str,
}

// Some fields's description can be found at
//
// https://dev.maxmind.com/geoip/docs/databases/city-and-country?lang=en#locations-files
#[derive(Default, Serialize)]
struct City<'a> {
    city_name: &'a str,
    continent_code: &'a str,
    country_code: &'a str,
    country_name: &'a str,
    timezone: &'a str,
    latitude: f64,
    longitude: f64,
    postal_code: &'a str,
    region_code: &'a str,
    region_name: &'a str,
    metro_code: String, // converted from u16 for consistency
}

impl FunctionTransform for Geoip {
    fn transform(&mut self, output: &mut OutputBuffer, mut events: Events) {
        events.for_each_log(|log| {
            let mut isp: Isp = Default::default();
            let mut city: City = Default::default();
            let ipaddress = log
                .get_field(self.source.as_str())
                .map(|s| s.to_string_lossy());

            if let Some(value) = &ipaddress {
                match FromStr::from_str(value) {
                    Ok(ip) => {
                        if self.has_isp_db() {
                            if let Ok(data) = self.database.lookup::<maxminddb::geoip2::Isp>(ip) {
                                if let Some(as_number) = data.autonomous_system_number {
                                    isp.autonomous_system_number = as_number as i64;
                                }

                                if let Some(as_organization) = data.autonomous_system_organization {
                                    isp.autonomous_system_organization = as_organization;
                                }

                                if let Some(isp_name) = data.isp {
                                    isp.isp = isp_name;
                                }

                                if let Some(organization) = data.organization {
                                    isp.organization = organization;
                                }
                            }
                        } else if let Ok(data) = self.database.lookup::<maxminddb::geoip2::City>(ip)
                        {
                            if let Some(city_names) = data.city.and_then(|c| c.names) {
                                if let Some(city_name) = city_names.get("en") {
                                    city.city_name = city_name;
                                }
                            }

                            if let Some(continent_code) = data.continent.and_then(|c| c.code) {
                                city.continent_code = continent_code;
                            }

                            if let Some(country) = data.country {
                                if let Some(country_code) = country.iso_code {
                                    city.country_code = country_code;
                                }

                                if let Some(country_name) = country
                                    .names
                                    .as_ref()
                                    .and_then(|names| names.get(&*self.locale))
                                {
                                    city.country_name = country_name;
                                }
                            }

                            if let Some(location) = data.location {
                                if let Some(time_zone) = location.time_zone {
                                    city.timezone = time_zone;
                                }
                                if let Some(latitude) = location.latitude {
                                    city.latitude = latitude
                                }
                                if let Some(longitude) = location.longitude {
                                    city.longitude = longitude
                                }
                                if let Some(metro_code) = location.metro_code {
                                    city.metro_code = metro_code.to_string();
                                }
                            }

                            // last subdivision is most specific per
                            // https://github.com/maxmind/GeoIP2-java/blob/39385c6ce645374039450f57208b886cf87ade47/src/main/java/com/maxmind/geoip2/model/AbstractCityResponse.java#L96-L107
                            if let Some(subdivision) =
                                data.subdivisions.as_ref().and_then(|s| s.last())
                            {
                                if let Some(name) = subdivision
                                    .names
                                    .as_ref()
                                    .and_then(|names| names.get(&*self.locale))
                                {
                                    city.region_name = name;
                                }
                                if let Some(iso_code) = subdivision.iso_code {
                                    city.region_code = iso_code;
                                }
                            }

                            if let Some(postal_code) = data.postal.and_then(|p| p.code) {
                                city.postal_code = postal_code;
                            }
                        }
                    }

                    Err(err) => {
                        error!(
                            message = "IP Address not parsed correctly",
                            %err,
                            address = value,
                            internal_log_rate_limit = true
                        )
                    }
                }
            } else {
                error!(
                    message = "Failed does not exist",
                    field = &self.source,
                    internal_log_rate_limit = true
                )
            }

            let json_value = if self.has_isp_db() {
                serde_json::to_value(isp)
            } else {
                serde_json::to_value(city)
            };

            if let Ok(value) = json_value {
                log.insert_field(self.target.as_str(), value);
            }
        });

        output.push(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transforms::transform_one;
    use event::fields;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    #[test]
    fn geoip() {
        let tests = vec![
            (
                "lookup success",
                fields!(
                    "remote_addr" => "2.125.160.216",
                    "request_path" => "/foo/bar"
                ),
                "tests/fixtures/geoip/GeoIP2-City-Test.mmdb",
                fields!(
                    "city_name" => "Boxford",
                    "country_code" => "GB",
                    "continent_code" => "EU",
                    "country_name" => "United Kingdom",
                    "region_code" => "WBK",
                    "region_name" => "West Berkshire",
                    "timezone" => "Europe/London",
                    "latitude" => 51.75,
                    "longitude" => -1.25,
                    "postal_code" => "OX1",
                    "metro_code" => ""
                ),
            ),
            (
                "partial result",
                fields!(
                    "remote_addr" => "67.43.156.9",
                    "request_path" => "/foo/bar",
                ),
                "tests/fixtures/geoip/GeoIP2-City-Test.mmdb",
                fields!(
                    "city_name" => "",
                    "country_code" => "BT",
                    "country_name" => "Bhutan",
                    "continent_code" => "AS",
                    "region_code" => "",
                    "region_name" => "",
                    "timezone" => "Asia/Thimphu",
                    "latitude" => 27.5,
                    "longitude" => 90.5,
                    "postal_code" => "",
                    "metro_code" => ""
                ),
            ),
            (
                "no results",
                fields!(
                    "remote_addr" => "10.1.12.1",
                    "request_path" => "/foo/bar",
                ),
                "tests/fixtures/geoip/GeoLite2-ASN-Test.mmdb",
                fields!(
                    "autonomous_system_number" => 0,
                    "autonomous_system_organization" => "",
                    "isp" => "",
                    "organization" => ""
                ),
            ),
        ];

        for (name, input, database, want) in tests {
            let database =
                maxminddb::Reader::open_readfile(database).expect("Open geoip database success");
            let mut transform = Geoip {
                database: Arc::new(database),
                source: "remote_addr".to_string(),
                target: "geo".to_string(),
                locale: "en".to_string(),
            };
            let event = transform_one(&mut transform, input).unwrap();

            for field in want.keys() {
                let key = format!("geo.{}", field);
                let got = event.as_log().get_field(key.as_str()).unwrap();
                assert_eq!(
                    got,
                    want.get(field).expect("field exists"),
                    "case: {}, field: {}",
                    name,
                    field
                )
            }
        }
    }
}
