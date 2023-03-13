/// Initialize the necessary bits needed to run a component test specification.
#[cfg(test)]
pub fn init_test() {
    framework::trace::init(false, false, "error", 10);
    testify::event::clear_recorded_events();
}
