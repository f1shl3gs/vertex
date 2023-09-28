use std::{cell::RefCell, collections::HashSet};

thread_local! {
    static EVENTS_RECORDED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

pub fn contains_name(name: &str) -> bool {
    EVENTS_RECORDED.with(|events| events.borrow().iter().any(|event| event.ends_with(name)))
}

pub fn clear_recorded_events() {
    EVENTS_RECORDED.with(|events| events.borrow_mut().clear())
}

#[allow(clippy::print_stdout)]
pub fn debug_print_events() {
    EVENTS_RECORDED.with(|events| {
        for event in events.borrow().iter() {
            println!("{}", event);
        }
    })
}

/// Record an emitted internal event. This is somewhat dumb at this point,
/// just recording the pure string value of the `emit!` call parameter.
/// At some point, making all internal events implement `Debug` or `Serialize`
/// might allow for more sophistication here, but this is good enough for
/// these tests. This should only be used by the test `emit!` macro. The
/// `check-events` script will test that emitted events contain the right
/// fields, etc
pub fn record_internal_event(event: &str) {
    // Remove leading '&'
    // Remove trailing '{fields}'
    let event = event.strip_prefix('&').unwrap_or(event);
    let event = event.find('{').map_or(event, |par| &event[..par]);
    EVENTS_RECORDED.with(|events| events.borrow_mut().insert(event.into()));
}

#[macro_export]
macro_rules! assert_event_data_eq {
    ($left:expr, $right:expr, $message:expr) => {{
        use event::EventDataEq as _;
        match (&($left), &($right)) {
            (left, right) => {
                if !left.event_data_eq(right) {
                    panic!(
                        "assertion failed: {}\n\n{}\n",
                        $message,
                        pretty_assertions::Comparison::new(left, right)
                    );
                }
            }
        }
    }};
    ($left:expr, $right:expr,) => {
        assert_event_data_eq!($left, $right)
    };
    ($left:expr, $right:expr) => {
        assert_event_data_eq!($left, $right, "`left.event_data_eq(right)`")
    };
}
