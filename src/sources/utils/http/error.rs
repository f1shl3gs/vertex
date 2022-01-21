#[derive(Debug, Serialize)]
pub struct ErrorMessage {
    code: u16,
    message: String,
}
