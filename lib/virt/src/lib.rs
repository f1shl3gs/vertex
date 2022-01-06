mod client;
mod error;
mod request;

pub use client::Client;
pub use error::Error;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
