pub fn trace_init() {
    #[cfg(unix)]
    let color = atty::is(atty::Stream::Stdout);
    // Windows: ANSI colors are not supported by cmd.ext
    // Color is false for everything except unix.
    #[cfg(not(unix))]
    let color = false;

    let levels = std::env::var("TEST_LOG").unwrap_or_else(|_| "warn".into());

    framework::trace::init(color, false, &levels, 10)
}
