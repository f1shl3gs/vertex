/// A component with a well-known name.
///
/// Users can derive this trait automatically by using the
///
pub trait NamedComponent {
    const NAME: &'static str;

    fn component_name(&self) -> &'static str {
        Self::NAME
    }
}
