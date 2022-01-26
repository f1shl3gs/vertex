/// Initialize the necessary bits needed to run a component test specification.
#[cfg(test)]
pub fn init_test() {
    crate::trace::test_init();
    testify::event::clear_recorded_events();

    // Handle multiple initialization
    let _ = internal::metric::init_global();
}
