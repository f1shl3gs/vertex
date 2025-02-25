#[cfg(all(test, feature = "kafka-integration-tests"))]
mod integration_tests {
    #[tokio::test]
    async fn healthcheck() {}
}
