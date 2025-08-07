use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use configurable::configurable_component;
use event::Events;
use event::log::OwnedTargetPath;
use event::log::path::parse_target_path;
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};
use value::{Value, path};

fn default_target() -> OwnedTargetPath {
    parse_target_path("geoip").unwrap()
}

fn default_locale() -> String {
    "en".to_string()
}

#[configurable_component(transform, name = "geoip")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Path to the `MaxMind GeoIP2` or `GeoLite2` binary city database file. Other
    /// databases, such as the country database, are not supported.
    ///
    /// GeoIP2: https://dev.maxmind.com/geoip/geoip2/downloadable
    /// GeoLite2: https://dev.maxmind.com/geoip/geoip2/geolite2/#Download_Access
    #[configurable(required)]
    database: PathBuf,

    /// The field to insert the resulting GeoIP data into.
    #[serde(default = "default_target")]
    #[configurable(required)]
    target: OwnedTargetPath,

    /// The field name that contains the IP address. This field should contain a
    /// valid IPv4 or IPv6 address.
    #[configurable(required, example = ".foo.bar[2]")]
    source: OwnedTargetPath,

    /// valid locales are: “de”, "en", “es”, “fr”, “ja”, “pt-BR”, “ru”, and “zh-CN” .
    ///
    /// https://dev.maxmind.com/geoip/docs/databases/city-and-country?lang=en
    #[serde(default = "default_locale")]
    locale: String,
}

#[async_trait]
#[typetag::serde(name = "geoip")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let database = maxminddb::Reader::mmap(&self.database)?;
        let metadata = database.metadata()?;
        let has_isp = metadata.database_type == ASN_TYPE || metadata.database_type == ISP_TYPE;

        Ok(Transform::function(Geoip {
            database: Arc::new(database),
            source: self.source.clone(),
            target: self.target.clone(),
            locale: self.locale.clone(),
            has_isp,
        }))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn enable_concurrency(&self) -> bool {
        true
    }
}

// MaxMind GeoIP database files have a type field we can use to recognize specific
// products. If we encounter one of these two types, we look for ASN/ISP information;
// otherwise we expect to be working with a City database.
const ASN_TYPE: &str = "GeoLite2-ASN";
const ISP_TYPE: &str = "GeoIP2-ISP";

#[derive(Clone)]
struct Geoip {
    database: Arc<maxminddb::Reader<maxminddb::Mmap>>,
    source: OwnedTargetPath,
    target: OwnedTargetPath,
    locale: String,
    has_isp: bool,
}

#[derive(Default)]
struct Isp<'a> {
    autonomous_system_number: i64,
    autonomous_system_organization: &'a str,
    isp: &'a str,
    organization: &'a str,
}

impl<'a> From<Isp<'a>> for Value {
    fn from(isp: Isp<'a>) -> Self {
        let mut value = Value::Object(BTreeMap::default());

        value.insert(
            path!("autonomous_system_number"),
            isp.autonomous_system_number,
        );
        value.insert(
            path!("autonomous_system_organization"),
            isp.autonomous_system_organization,
        );
        value.insert(path!("isp"), isp.isp);
        value.insert(path!("organization"), isp.organization);

        value
    }
}

// Some fields's description can be found at
//
// https://dev.maxmind.com/geoip/docs/databases/city-and-country?lang=en#locations-files
#[derive(Default)]
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

impl<'a> From<City<'a>> for Value {
    fn from(city: City<'a>) -> Self {
        let mut value = Value::Object(BTreeMap::default());

        value.insert(path!("city_name"), city.city_name);
        value.insert(path!("continent_code"), city.continent_code);
        value.insert(path!("country_code"), city.country_code);
        value.insert(path!("country_name"), city.country_name);
        value.insert(path!("timezone"), city.timezone);
        value.insert(path!("latitude"), city.latitude);
        value.insert(path!("longitude"), city.longitude);
        value.insert(path!("postal_code"), city.postal_code);
        value.insert(path!("region_code"), city.region_code);
        value.insert(path!("region_name"), city.region_name);
        value.insert(path!("metro_code"), city.metro_code);

        value
    }
}

impl FunctionTransform for Geoip {
    fn transform(&mut self, output: &mut OutputBuffer, mut events: Events) {
        events.for_each_log(|log| {
            let mut isp: Isp = Default::default();
            let mut city: City = Default::default();
            let ipaddress = log.get(&self.source).map(|s| s.to_string_lossy());

            if let Some(value) = &ipaddress {
                match FromStr::from_str(value) {
                    Ok(ip) => {
                        if self.has_isp {
                            if let Ok(data) = self.database.lookup::<maxminddb::Isp>(ip) {
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
                        } else if let Ok(data) = self.database.lookup::<maxminddb::City>(ip) {
                            if let Some(city_names) = data.city.and_then(|c| c.names)
                                && let Some(city_name) = city_names
                                    .iter()
                                    .find_map(|(key, value)| (*key == "en").then_some(*value))
                            {
                                city.city_name = city_name;
                            }

                            if let Some(continent_code) = data.continent.and_then(|c| c.code) {
                                city.continent_code = continent_code;
                            }

                            if let Some(country) = data.country {
                                if let Some(country_code) = country.iso_code {
                                    city.country_code = country_code;
                                }

                                if let Some(country_name) = country.names.and_then(|names| {
                                    names.iter().find_map(|(key, value)| {
                                        (*key == self.locale).then_some(*value)
                                    })
                                }) {
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
                                if let Some(name) = subdivision.names.as_ref().and_then(|names| {
                                    names.iter().find_map(|(key, value)| {
                                        (*key == self.locale).then_some(*value)
                                    })
                                }) {
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
                            address = ?value,
                            internal_log_rate_limit = true
                        )
                    }
                }
            } else {
                error!(
                    message = "Failed does not exist",
                    field = ?self.source,
                    internal_log_rate_limit = true
                )
            }

            if self.has_isp {
                log.insert(&self.target, isp);
            } else {
                log.insert(&self.target, city);
            }
        });

        output.push(events)
    }
}

#[cfg(test)]
mod tests {
    use value::value;

    use super::*;
    use crate::transforms::transform_one;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    #[test]
    fn geoip() {
        let tests = vec![
            (
                "lookup success",
                value!({
                    "remote_addr": "2.125.160.216",
                    "request_path": "/foo/bar"
                }),
                "tests/geoip/GeoIP2-City-Test.mmdb",
                value!({
                    "city_name": "Boxford",
                    "country_code": "GB",
                    "continent_code": "EU",
                    "country_name": "United Kingdom",
                    "region_code": "WBK",
                    "region_name": "West Berkshire",
                    "timezone": "Europe/London",
                    "latitude": 51.75,
                    "longitude": (-1.25),
                    "postal_code": "OX1",
                    "metro_code": ""
                }),
            ),
            (
                "partial result",
                value!({
                    "remote_addr": "67.43.156.9",
                    "request_path": "/foo/bar",
                }),
                "tests/geoip/GeoIP2-City-Test.mmdb",
                value!({
                    "city_name": "",
                    "country_code": "BT",
                    "country_name": "Bhutan",
                    "continent_code": "AS",
                    "region_code": "",
                    "region_name": "",
                    "timezone": "Asia/Thimphu",
                    "latitude": 27.5,
                    "longitude": 90.5,
                    "postal_code": "",
                    "metro_code": ""
                }),
            ),
            (
                "no results",
                value!({
                    "remote_addr": "10.1.12.1",
                    "request_path": "/foo/bar",
                }),
                "tests/geoip/GeoLite2-ASN-Test.mmdb",
                value!({
                    "autonomous_system_number": 0,
                    "autonomous_system_organization": "",
                    "isp": "",
                    "organization": ""
                }),
            ),
        ];

        for (name, input, database, want) in tests {
            let database = maxminddb::Reader::mmap(database).expect("Open geoip database success");
            let metadata = database.metadata().unwrap();
            let has_isp = metadata.database_type == ASN_TYPE || metadata.database_type == ISP_TYPE;
            let mut transform = Geoip {
                database: Arc::new(database),
                source: parse_target_path("remote_addr").unwrap(),
                target: parse_target_path("geo").unwrap(),
                locale: "en".to_string(),
                has_isp,
            };
            let event = transform_one(&mut transform, input).unwrap();

            assert_eq!(
                event.as_log().value().get("geo").unwrap(),
                &want,
                "test: {name}"
            );
        }
    }
}
