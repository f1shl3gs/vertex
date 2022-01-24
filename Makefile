VERSION ?= $(shell cat Cargo.toml | grep '^version = ' | grep -Po '\d+.\d+.\d+')

artifacts-dir:
	mkdir -p target/artifacts

build-timing:
	cargo +nightly build -p vertex --bin vertex -Z timings --release

bloat:
	cargo bloat --release --crates -n 50

lines:
	@./scripts/lines.sh

hooks:
	ln -sf ../../scripts/pre-commit.sh .git/hooks/pre-commit

.PHONY: fmt
fmt:
	cargo fmt

build_x86_64-unknown-linux-musl:
	@docker build -f ci/cross/x86_64-unknown-linux-musl.dockerfile -t vertex-cross:x86_64-unknown-linux-musl ci/cross
	@cross build \
		--release \
		--target x86_64-unknown-linux-musl \
		--no-default-features \
		--features target-x86_64-unknown-linux-musl

build_x86_64-unknown-linux-gnu: artifacts-dir
	docker build -f ci/cross/x86_64-unknown-linux-gnu.dockerfile -t vertex-cross:x86_64-unknown-linux-gnu ci/cross
	cross build \
		--release \
		--no-default-features \
		--target x86_64-unknown-linux-gnu \
		--features target-x86_64-unknown-linux-gnu
	cp target/x86_64-unknown-linux-gnu/release/vertex target/artifacts/vertex-x86_64-unknown-linux-gnu

# Integration tests
.PHONY: integration-test-nginx_stub
integration-test-nginx_stub:
	cargo test -p vertex --lib sources::nginx_stub::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: integration-test-redis
integration-test-redis:
	cargo test -p vertex --lib sources::redis::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: integration-test-zookeeper
integration-test-zookeeper:
	cargo test -p vertex --lib sources::zookeeper::integration_tests --features all-integration-tests --no-fail-fast

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

.PHONY: integration-tests-prometheus_remote_write
integration-test-prometheus_remote_write:
	cargo test -p vertex --lib sources::prometheus_remote_write::integration_tests --features all-integration-tests --no-fail-fast

.PHONY: integration-tests
integration-tests: integration-test-*

.PHONY: test
test:
	cargo test --workspace --no-fail-fast

.PHONY: check_clippy
check_clippy:
	cargo clippy --workspace --all-targets --features all-integration-tests -- -A warnings

.PHONY: check_shell
check_shell:
	bash ci/check-scripts.sh

.PHONY: check_fmt
check_fmt:
	cargo fmt -- --check

.PHONY: check
check: check_shell check_clippy check_fmt

.PHONY: bench-vertex
bench-vertex:
	cargo bench --features bench --no-default-features --features benches

.PHONY: bench-prometheus
bench-prometheus:
	cargo bench --manifest-path lib/prometheus/Cargo.toml

.PHONY: bench-tracing-limit
bench-tracing-limit:
	cargo bench --manifest-path lib/tracing-limit/Cargo.toml

.PHONY: bench-humanize
bench-humanize:
	cargo bench --manifest-path lib/humanize/Cargo.toml

# profile when bench
# cargo bench --bench hwmon_gather -- --profile-time=30