#[cfg(all(test, feature = "integration-tests-kafka"))]
mod integration_tests {
    use testify::random::random_string;

    #[tokio::test]
    async fn healthcheck() {
        crate::trace::test_init();
        let topic = format!("test-{}", random_string(10));
    }
}
