use crate::GenerateError;

/// Well-known validator formats as described in the [JSON Schema Validation specification][jsvs].
///
/// Not all defined formats are present here.
pub enum Format {
    /// A date.
    ///
    /// Conforms to the `full-date` production as outlined in [RFC 3339, section 5.6][rfc3339], and specified in the
    /// [JSON Schema Validation specification, section 7.3.1][jsvs].
    ///
    /// [rfc3339]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6
    /// [jsvs]: https://datatracker.ietf.org/doc/html/draft-handrews-json-schema-validation-02#section-7.3.1
    Date,
    /// A time.
    ///
    /// Conforms to the `full-time` production as outlined in [RFC 3339, section 5.6][rfc3339], and specified in the
    /// [JSON Schema Validation specification, section 7.3.1][jsvs].
    ///
    /// [rfc3339]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6
    /// [jsvs]: https://datatracker.ietf.org/doc/html/draft-handrews-json-schema-validation-02#section-7.3.1
    Time,
    /// A datetime.
    ///
    /// Conforms to the `date-time` production as outlined in [RFC 3339, section 5.6][rfc3339], and specified in the
    /// [JSON Schema Validation specification, section 7.3.1][jsvs].
    ///
    /// [rfc3339]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6
    /// [jsvs]: https://datatracker.ietf.org/doc/html/draft-handrews-json-schema-validation-02#section-7.3.1
    DateTime,
    /// An email address.
    ///
    /// Conforms to the `addr-spec` production as outlined in [RFC 5322, section 3.4.1][rfc5322], and specified in the
    /// [JSON Schema Validation specification, section 7.3.2][jsvs].
    ///
    /// [rfc5322]: https://datatracker.ietf.org/doc/html/rfc5322#section-3.4.1
    /// [jsvs]: https://datatracker.ietf.org/doc/html/draft-handrews-json-schema-validation-02#section-7.3.2
    Email,
    /// A uniform resource identifier (URI).
    ///
    /// Conforms to the `URI` production as outlined in [RFC 3986, appendix A][rfc3986], and specified in the [JSON
    /// Schema Validation specification, section 7.3.5][jsvs].
    ///
    /// [rfc3986]: https://datatracker.ietf.org/doc/html/rfc3986#appendix-A
    /// [jsvs]: https://datatracker.ietf.org/doc/html/draft-handrews-json-schema-validation-02#section-7.3.5
    Uri,
    /// An IPv4 address.
    ///
    /// Conforms to the `dotted-quad` production as outlined in [RFC 2673, section 3.2][rfc2673], and specified in the
    /// [JSON Schema Validation specification, section 7.3.4][jsvs].
    ///
    /// [rfc2673]: https://datatracker.ietf.org/doc/html/rfc2673#section-3.2
    /// [jsvs]: https://datatracker.ietf.org/doc/html/draft-handrews-json-schema-validation-02#section-7.3.4
    IPv4,
    /// An IPv6 address.
    ///
    /// Conforms to the "conventional text forms" as outlined in [RFC 4291, section 2.2][rfc4291], and specified in the
    /// [JSON Schema Validation specification, section 7.3.4][jsvs].
    ///
    /// [rfc4291]: https://datatracker.ietf.org/doc/html/rfc4291#section-2.2
    /// [jsvs]: https://datatracker.ietf.org/doc/html/draft-handrews-json-schema-validation-02#section-7.3.4
    IPv6,
    /// A universally unique identifier (UUID).
    ///
    /// Conforms to the `UUID` production as outlined in [RFC 4122, section 3][rfc4122], and specified in the
    /// [JSON Schema Validation specification, section 7.3.5][jsvs].
    ///
    /// [rfc4122]: https://datatracker.ietf.org/doc/html/rfc4122#section-3
    /// [jsvs]: https://datatracker.ietf.org/doc/html/draft-handrews-json-schema-validation-02#section-7.3.5
    Uuid,
    /// A regular expression.
    ///
    /// Conforms to the specification as outlined in [ECMA 262][emca262], and specified in the
    /// [JSON Schema Validation specification, section 7.3.8][jsvs].
    ///
    /// [emca262]: https://www.ecma-international.org/publications-and-standards/standards/ecma-262/
    /// [jsvs]: https://datatracker.ietf.org/doc/html/draft-handrews-json-schema-validation-02#section-7.3.8
    Regex,
}

impl TryFrom<&'static str> for Format {
    type Error = GenerateError;

    fn try_from(value: &'static str) -> Result<Self, Self::Error> {
        let format = match value {
            "date" => Format::Date,
            "time" => Format::Time,
            "date-time" => Format::DateTime,
            "email" => Format::Email,
            "uri" => Format::Uri,
            "ipv4" => Format::IPv4,
            "ipv6" => Format::IPv6,
            "uuid" => Format::Uuid,
            "regex" => Format::Regex,
            _ => return Err(GenerateError::UnknownFormat(value)),
        };

        Ok(format)
    }
}
