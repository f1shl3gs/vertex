pub fn trace_init() {
    #[cfg(unix)]
    let color = atty::is(atty::Stream::Stdout);
    #[cfg(not(unix))]
    let color = false;

    let level = std::env::var("TEST_LOG").unwrap_or_else(|_| "info".to_string());

    framework::trace::init(color, false, &level);
}
