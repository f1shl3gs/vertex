use std::io::ErrorKind;
use std::path::Path;
use std::str::FromStr;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU32, Ordering};

use event::{EventStatus, Finalizable};
use tracing_fluent_assertions::{AssertionRegistry, AssertionsLayer};
use tracing_subscriber::{Layer, Registry, filter::LevelFilter, layer::SubscriberExt};

#[macro_export]
macro_rules! assert_file_does_not_exist_async {
    ($file_path:expr) => {{
        let result = tokio::fs::metadata($file_path).await;
        assert!(result.is_err());
        assert_eq!(
            std::io::ErrorKind::NotFound,
            result.expect_err("is_err() was true").kind(),
            "got unexpected error kind"
        );
    }};
}

#[macro_export]
macro_rules! assert_file_exists_async {
    ($file_path:expr) => {{
        let result = tokio::fs::metadata($file_path).await;
        assert!(result.is_ok());
        assert!(
            result.expect("is_ok() was true").is_file(),
            "path exists but is not file"
        );
    }};
}

#[macro_export]
macro_rules! await_timeout {
    ($fut:expr, $secs:expr) => {{
        tokio::time::timeout(std::time::Duration::from_secs($secs), $fut)
            .await
            .expect("future should not timeout")
    }};
}

pub static INTERNAL_COUNTER: AtomicU32 = AtomicU32::new(0);

pub async fn with_temp_dir<F, Fut, V>(f: F) -> V
where
    F: FnOnce(&Path) -> Fut,
    Fut: Future<Output = V>,
{
    let prefix = "vertex-buffers";

    let tmp_dir = loop {
        let path = std::env::temp_dir().join(format!(
            "{}/{:x}-{:x}",
            prefix,
            std::process::id(),
            INTERNAL_COUNTER.fetch_add(1, Ordering::AcqRel)
        ));

        if let Ok(_n) = std::fs::create_dir_all(&path) {
            break path;
        }

        match std::fs::create_dir(&path) {
            Ok(()) => break path,
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                // dir already exists, just retry
            }
            Err(err) => {
                panic!("Failed to create temp dir {:?}: {}", path, err);
            }
        }
    };

    f(tmp_dir.as_path()).await
}

pub fn install_tracing_helpers() -> AssertionRegistry {
    // TODO: This installs the assertions layer globally, so all tests will run through it.  Since
    // most of the code being tested overlaps, individual tests should wrap their async code blocks
    // with a unique span that can be matched on specifically with
    // `AssertionBuilder::with_parent_name`.
    //
    // TODO: We also need a better way of wrapping our test functions in their own parent spans, for
    // the purpose of isolating their assertions.  Right now, we do it with a unique string that we
    // have set to the test function name, but this is susceptible to being copypasta'd
    // unintentionally, thus letting assertions bleed into other tests.
    //
    // Maybe we should add a helper method to `tracing-fluent-assertions` for generating a
    // uniquely-named span that can be passed directly to the assertion builder methods, then it's a
    // much tighter loop.
    //
    // TODO: At some point, we might be able to write a simple derive macro that does this for us, and
    // configures the other necessary bits, but for now.... by hand will get the job done.
    static ASSERTION_REGISTRY: LazyLock<AssertionRegistry> = LazyLock::new(|| {
        let assertion_registry = AssertionRegistry::default();
        let assertions_layer = AssertionsLayer::new(&assertion_registry);

        // Constrain the actual output layer to the normal RUST_LOG-based control mechanism, so that
        // assertions can run unfettered but without also spamming the console with logs.
        let fmt_filter = std::env::var("RUST_LOG")
            .map_err(|_| ())
            .and_then(|s| LevelFilter::from_str(s.as_str()).map_err(|_| ()))
            .unwrap_or(LevelFilter::OFF);
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_ansi(true)
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
            .with_test_writer()
            .with_filter(fmt_filter);

        let base_subscriber = Registry::default();
        let subscriber = base_subscriber.with(assertions_layer).with(fmt_layer);

        tracing::subscriber::set_global_default(subscriber).unwrap();
        assertion_registry
    });

    ASSERTION_REGISTRY.clone()
}

pub(crate) async fn acknowledge(mut event: impl Finalizable) {
    event
        .take_finalizers()
        .update_status(EventStatus::Delivered);
    // Finalizers are implicitly dropped here, sending the status update.
    tokio::task::yield_now().await;
}
