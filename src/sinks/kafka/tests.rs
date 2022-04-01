#[cfg(all(test, feature = "integration-tests-kafka"))]
mod integration_tests {
    #[tokio::test]
    async fn healthcheck() {}
}
