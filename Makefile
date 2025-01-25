VERSION ?= $(shell cat Cargo.toml | grep '^version = ' | grep -Po '\d+.\d+.\d+')

build:
	cargo build --release

dev:
	cargo build

clean:
	rm -rf target

x86_64-unknown-linux-musl:
	@cross build \
		--release \
		--target $@ \
		--no-default-features \
		--features target-$@

x86_64-unknown-linux-gnu:
	@cross build \
		--release \
		--no-default-features \
		--target $@ \
		--features target-$@

build-timing: clean
	cargo build --release --timings

bloat:
	cargo bloat --release --crates --target x86_64-unknown-linux-gnu -n 200

lines:
	@./scripts/lines.sh

hooks:
	ln -sf ../../scripts/pre-commit.sh .git/hooks/pre-commit

.PHONY: fmt
fmt:
	cargo fmt

## Build Container
.PHONY: builder-x86_64-unknown-linux-musl
builder-x86_64-unknown-linux-musl:
	docker build -f ci/cross/x86_64-unknown-linux-musl.dockerfile -t vertex-cross:x86_64-unknown-linux-musl ci/cross

.PHONY: builder-x86_64-unknown-linux-gnu
builder-x86_64-unknown-linux-gnu:
	docker build -f ci/cross/x86_64-unknown-linux-gnu.dockerfile -t vertex-cross:x86_64-unknown-linux-gnu ci/cross

## Integration tests
.PHONY: integration-test-nginx_stub
integration-test-nginx_stub:
	cargo test -p vertex --lib sources::nginx_stub::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: redis-integration-tests
redis-integration-tests:
	cargo test -p vertex --lib sources::redis::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: integration-test-zookeeper
integration-test-zookeeper:
	cargo test -p vertex --lib sources::zookeeper::integration_tests --features all-integration-tests --no-fail-fast -- --nocapture

.PHONY: integration-test-memcached
integration-test-memcached:
	cargo test -p vertex --lib sources::memcached::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: integration-test-haproxy
integration-test-haproxy:
	cargo test -p vertex --lib sources::haproxy::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: integration-test-mysql
integration-test-mysql:
	cargo test -p vertex --lib sources::mysqld::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: integration-test-consul
integration-test-consul:
	cargo test -p vertex --lib sources::consul::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: integration-test-prometheus_remote_write
integration-test-prometheus_remote_write:
	cargo test -p vertex --lib sinks::prometheus_remote_write::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: integration-test-loki
integration-test-loki:
	cargo test -p vertex --lib sinks::loki::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: integration-test-kafka
integration-test-kafka:
	cargo test -p vertex --lib sources::kafka::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: integration-test-elasticsearch
integration-test-elasticsearch:
	cargo test -p vertex --lib sinks::elasticsearch::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: redfish-integration-tests
redfish-integration-tests:
	bash ./scripts/redfish_prepare.sh
	cargo test -p vertex --lib sources::redfish::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: clickhouse-integration-tests
clickhouse-integration-tests:
	cargo test -p vertex --lib sources::clickhouse_metrics::integration_tests --features all-integration-tests --no-fail-fast -- --nocapture

.PHONY: fluent-integration-tests
fluent-integration-tests:
	cargo test -p vertex --lib sources::fluent::integration_tests --features all-integration-tests --no-fail-fast -- --nocapture

.PHONY: integration-tests
integration-tests: integration-test-consul integration-test-haproxy integration-test-memcached integration-test-mysql integration-test-nginx_stub redis-integration-tests integration-test-zookeeper integration-test-prometheus_remote_write integration-test-elasticsearch redfish-integration-tests fluent-integration-tests

.PHONY: udeps
udeps:
	cargo machete

.PHONY: doc-test
doc-test:
	cargo test --doc --workspace

.PHONY: test
test:
	cargo nextest run --workspace --no-fail-fast

.PHONY: check_clippy
check_clippy:
	cargo clippy --workspace --all-targets --features all-integration-tests -- -D warnings

.PHONY: check_shell
check_shell:
	bash ci/check-scripts.sh

.PHONY: check_fmt
check_fmt:
	cargo fmt -- --check

.PHONY: check
check: check_shell check_clippy check_fmt

.PHONY: bench-event
bench-event:
	cargo bench --manifest-path lib/event/Cargo.toml

.PHONY: bench-vertex
bench-vertex:
	cargo bench --no-default-features --features benches

.PHONY: bench-prometheus
bench-prometheus:
	cargo bench --manifest-path lib/prometheus/Cargo.toml

.PHONY: bench-tracing-limit
bench-tracing-limit:
	cargo bench --manifest-path lib/tracing-limit/Cargo.toml

.PHONY: bench-metrics
bench-metrics:
	cargo bench --manifest-path lib/metrics/Cargo.toml

.PHONY: images
images: x86_64-unknown-linux-gnu
	cp target/x86_64-unknown-linux-gnu/release/vertex distribution/docker/distroless-libc
	cd distribution/docker/distroless-libc && docker build -t f1shl3gs/vertex:nightly-distroless .

.PHONY: kind_load
kind_load: images
	kind load docker-image f1shl3gs/vertex:nightly-distroless

.PHONY: regression
regression: build
	docker build -f regression/Dockerfile  -t vertex:regression .
	cd regression/$(CASE) && docker-compose -f ../docker-compose.yaml --abort-on-container-exit up


# profile when bench
# cargo bench --bench hwmon_gather -- --profile-time=30
