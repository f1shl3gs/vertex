//! # hyper-rustls
//!
//! A pure-Rust HTTPS connector for [hyper](https://hyper.rs), based on
//! [Rustls](https://github.com/ctz/rustls).
//!
//! ## Example
//!
//! ```no_run
//! # #[cfg(all(feature = "rustls-native-certs", feature = "tokio-runtime", feature = "http1"))]
//! # fn main() {
//! use hyper::{Body, Client, StatusCode, Uri};
//!
//! let mut rt = tokio::runtime::Runtime::new().unwrap();
//! let url = ("https://hyper.rs").parse().unwrap();
//! let https = hyper_rustls::HttpsConnectorBuilder::new()
//!     .with_native_roots()
//!     .https_only()
//!     .enable_http1()
//!     .build();
//!
//! let client: Client<_, hyper::Body> = Client::builder().build(https);
//!
//! let res = rt.block_on(client.get(url)).unwrap();
//! assert_eq!(res.status(), StatusCode::OK);
//! # }
//! # #[cfg(not(all(feature = "rustls-native-certs", feature = "tokio-runtime", feature = "http1")))]
//! # fn main() {}
//! ```

#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use config::ConfigBuilderExt;
pub use connector::builder::ConnectorBuilder as HttpsConnectorBuilder;
pub use connector::HttpsConnector;
pub use stream::MaybeHttpsStream;

mod config;
mod connector;
mod stream;

/// The various states of the [HttpsConnectorBuilder]
// pub mod builder_states {
//     pub use super::builder::WantsProtocols3;
//     pub use super::builder::{
//         WantsProtocols1, WantsProtocols2, WantsSchemes, WantsTlsConfig
//     };
// }

#[cfg(test)]
mod tests {
    #[test]
    fn dummy() {

        println!("abc");
    }
}