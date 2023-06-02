use std::io::IsTerminal;

pub fn trace_init() {
    #[cfg(unix)]
    let color = std::io::stdout().is_terminal();
    #[cfg(not(unix))]
    let color = false;

    let level = std::env::var("TEST_LOG").unwrap_or_else(|_| "info".to_string());

    framework::trace::init(color, false, &level, 10);
}
