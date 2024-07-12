use bytes::Bytes;
use http::{Response, StatusCode};
use http_body_util::Full;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

#[derive(Debug)]
pub struct ErrorMessage {
    pub code: StatusCode,
    pub message: String,
}

impl Serialize for ErrorMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut em = serializer.serialize_struct("error_message", 2).unwrap();
        em.serialize_field("code", &self.code.as_u16()).unwrap();
        em.serialize_field("message", self.message.as_str())
            .unwrap();
        em.end()
    }
}

impl ErrorMessage {
    pub fn new(code: StatusCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<ErrorMessage> for Response<Full<Bytes>> {
    fn from(err: ErrorMessage) -> Self {
        Response::builder()
            .status(err.code)
            .body(Full::new(Bytes::from(err.message)))
            .unwrap()
    }
}
