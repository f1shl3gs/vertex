use std::io::Read;

use bytes::{Buf, Bytes};
use flate2::read::{MultiGzDecoder, ZlibDecoder};
use http::StatusCode;
use snap::raw::Decoder as SnappyDecoder;

use super::ErrorMessage;

pub fn decode(encodings: Option<&str>, mut body: Bytes) -> Result<Bytes, ErrorMessage> {
    if let Some(encodings) = encodings {
        for encoding in encodings.rsplit(',').map(str::trim) {
            body = match encoding {
                "identity" => body,
                "gzip" => {
                    let mut decoded = Vec::new();
                    MultiGzDecoder::new(body.reader())
                        .read_to_end(&mut decoded)
                        .map_err(|err| handle_decode_error(encoding, err))?;
                    decoded.into()
                }
                "deflate" => {
                    let mut decoded = Vec::new();
                    ZlibDecoder::new(body.reader())
                        .read_to_end(&mut decoded)
                        .map_err(|err| handle_decode_error(encoding, err))?;
                    decoded.into()
                }
                "snappy" => SnappyDecoder::new()
                    .decompress_vec(&body)
                    .map_err(|err| handle_decode_error(encoding, err))?
                    .into(),
                encoding => {
                    return Err(ErrorMessage::new(
                        StatusCode::UNSUPPORTED_MEDIA_TYPE,
                        format!("Unsupported encoding {}", encoding),
                    ));
                }
            }
        }
    }

    Ok(body)
}

#[inline]
fn handle_decode_error(encoding: &str, err: impl std::error::Error) -> ErrorMessage {
    // TODO: metrics
    // counter!("http_decompress_error_total", 1, "encoding" => encoding.to_string());

    ErrorMessage::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        format!(
            "Failed decompressing payload with {} decoder, err: {}.",
            encoding, err
        ),
    )
}
