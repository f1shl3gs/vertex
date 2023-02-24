use crate::errors::GenerateError;
use crate::schema;

/// A type that can be represented in a Vertex configuration.
///
/// In vertex, we want to be able to generate a schema for our configuration
/// such that we can have a Rust-agnostic definition of exactly what is
/// configurable, what values are allowed, what bounds exist, and so on and
/// forth.
///
/// `Configurable` provides the machinery to allow describing and encoding
/// the shape of a type, recursively, so that by instrumenting all transitive
/// types of the configuration, the schema can be discovered by generating
/// the schema from some root type.
pub trait Configurable
where
    Self: Sized,
{
    /// Gets the reference name of this value, if any.
    ///
    /// When specified, this implies the value is both complex and standardized,
    /// and should be reused within any generated schema it is present in.
    fn reference() -> Option<&'static str> {
        None
    }

    /// Whether or not this value is required.
    fn required() -> bool {
        false
    }

    /// Generate the schema for this value.
    ///
    /// If an error occurs while generating the schema, an error variant will
    /// be returned describing the issue.
    fn generate_schema(
        gen: &mut schema::SchemaGenerator,
    ) -> Result<schema::SchemaObject, GenerateError>;
}

pub trait ConfigurableString: Configurable {}

impl ConfigurableString for String {}
