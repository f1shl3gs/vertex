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
	cargo bloat --release --crates --target x86_64-unknown-linux-gnu -n 200 | tee bloat.txt

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

# Integration tests
.PHONY: redis-integration-tests
redis-integration-tests:
	cargo test -p vertex --lib sources::redis::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: haproxy-integration-tests
haproxy-integration-tests:
	cargo test -p vertex --lib sources::haproxy::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: consul-integration-tests
consul-integration-tests:
	cargo test -p vertex --lib sources::consul::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: prometheus_remote_write-integration-tests
prometheus_remote_write-integration-tests:
	cargo test -p vertex --lib sinks::prometheus_remote_write::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: pulsar_integration-tests
pulsar_integration-tests:
	cargo test -p vertex --lib sources::pulsar::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: loki-integration-tests
loki-integration-tests:
	cargo test -p vertex --lib sinks::loki::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: kafka-integration-tests
kafka-integration-tests:
	cargo test -p vertex --lib sources::kafka::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: elasticsearch-integration-tests
elasticsearch-integration-tests:
	cargo test -p vertex --lib sinks::elasticsearch::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: dnstap-integration-tests
dnstap-integration-tests:
	cargo test -p vertex --lib sources::dnstap::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: memcached-integration-tests
memcached-integration-tests:
	cargo test -p vertex --lib sources::memcached::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: mysql-integration-tests
mysql-integration-tests:
	cargo test -p vertex --lib sources::mysqld::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: nginx_stub-integration-tests
nginx_stub-integration-tests:
	cargo test -p vertex --lib sources::nginx_stub::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: redfish-integration-tests
redfish-integration-tests:
	# bash ./scripts/redfish_prepare.sh
	# cargo test -p vertex --lib sources::redfish::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: clickhouse-integration-tests
clickhouse-integration-tests:
	cargo test -p vertex --lib sources::clickhouse_metrics::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: fluent-integration-tests
fluent-integration-tests:
	cargo test -p vertex --lib sources::fluent::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: zookeeper-integration-tests
zookeeper-integration-tests:
	cargo test -p vertex --lib sources::zookeeper::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: integration-tests
integration-tests: redis-integration-tests haproxy-integration-tests consul-integration-tests
integration-tests: loki-integration-tests prometheus_remote_write-integration-tests kafka-integration-tests
integration-tests: elasticsearch-integration-tests dnstap-integration-tests memcached-integration-tests
integration-tests: mysql-integration-tests nginx_stub-integration-tests redfish-integration-tests
integration-tests: clickhouse-integration-tests fluent-integration-tests zookeeper-integration-tests
integration-tests: pulsar_integration-tests

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
	cargo bench --no-default-features \
		--features sources-haproxy \
		--features sources-node \
		--features sinks-loki

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
	cp target/x86_64-unknown-linux-gnu/release/vertex distribution/docker/distroless
	cd distribution/docker/distroless && docker build -t f1shl3gs/vertex:nightly-distroless .

.PHONY: regression
regression: build
	docker build -f regression/Dockerfile  -t vertex:regression .
	cd regression/$(CASE) && docker-compose -f ../docker-compose.yaml up --abort-on-container-exit

.PHONY: chiseled
chiseled:
	cd distribution/docker/chiseled && docker build -t chiseled ./

.PHONY: deploy-dev
deploy-dev: build
	strip target/release/vertex && cp target/release/vertex distribution/docker/kind/vertex
	cd distribution/docker/kind && docker build -t f1shl3gs/vertex:nightly-distroless .
	kind load docker-image f1shl3gs/vertex:nightly-distroless --name dev
