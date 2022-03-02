/// A k8s client configuration
///
/// This type is designed to hold all possible variants of the configuration.
/// It also abstracts the client from the various ways to obtain the
/// configuration.
///
/// The implementation is fairly limited, and only covers the use cases we
/// support.
#[derive(Clone, Debug)]
pub struct Config {}
