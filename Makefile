VERSION ?= $(shell cat Cargo.toml | grep '^version = ' | grep -Po '\d+.\d+.\d+')

artifacts-dir:
	mkdir -p target/artifacts

build:
	cargo build --release
	# striping is not enabled in stable Cargo, so here we are
	strip target/release/vertex

build-musl:
	cargo build --release --target=x86_64-unknown-linux-musl

build-timing:
	cargo +nightly build -p vertex --bin vertex -Z timings --release

bloat:
	cargo bloat --release --crates

lines:
	@./scripts/lines.sh

static:
	docker run --rm -it -v "/home/f1shl3gs/Workspaces/clion/vertex/docker/builder/cargo-config.toml:/opt/rust/cargo/config" -v "$$(pwd)":/home/rust/src musl-builder cargo build --release

# archives
target/artifacts/vector-${VERSION}:


build_x86_64-unknown-linux-musl:
	podman build -f ci/cross/x86_64-unknown-linux-musl.dockerfile -t vertex-cross:x86_64-unknown-linux-musl ci/cross
	cross build \
		--release \
		--no-default-features \
		--target x86_64-unknown-linux-musl \
		--features target-x86_64-unknown-linux-musl

build_x86_64-unknown-linux-gnu: artifacts-dir
	podman build -f ci/cross/x86_64-unknown-linux-gnu.dockerfile -t vertex-cross:x86_64-unknown-linux-gnu ci/cross
	cross build \
		--release \
		--no-default-features \
		--target x86_64-unknown-linux-gnu \
		--features target-x86_64-unknown-linux-gnu
	cp target/x86_64-unknown-linux-gnu/release/vertex target/artifacts/vertex-x86_64-unknown-linux-gnu

# Updates the Cargo config to product disk optimized builds(for CI, not users)
.PHONY: slim-builds
slim-builds:
	./ci/slim-builds.sh

# Integration tests
integration-test-nginx_stub: slim-builds
	cargo test -p vertex --lib sources::nginx_stub::integration_tests:: --features integration-tests-nginx_stub --no-fail-fast

integration-test-redis: slim-builds
	cargo test -p vertex --lib sources::redis::integration_tests:: --features integration-tests-redis --no-fail-fast

integration-test-zookeeper: slim-builds
	cargo test -p vertex --lib sources::zookeeper::integration_tests:: --features integration-tests-zookeeper --no-fail-fast

integration-test-memcached: slim-builds
	cargo test -p vertex --lib sources::memcached::integration_tests:: --features integration-tests-memcached --no-fail-fast

integration-test-haproxy: slim-builds
	cargo test -p vertex --lib sources::haproxy::integration_tests:: --features integration-tests-haproxy --no-fail-fast

# profile when bench
# cargo bench --bench hwmon_gather -- --profile-time=30