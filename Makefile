VERSION ?= $(shell cat Cargo.toml | grep '^version = ' | grep -Po '\d+.\d+.\d+')

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
	docker build -f ci/cross/x86_64-unknown-linux-musl.dockerfile -t vertex-cross:x86_64-unknown-linux-musl ci/cross
	cross build \
		--release \
		--no-default-features \
		--target x86_64-unknown-linux-musl \
		--features target-x86_64-unknown-linux-musl

build_x86_64-unknown-linux-gnu:
	docker build -f ci/cross/x86_64-unknown-linux-gnu.dockerfile -t vertex-cross:x86_64-unknown-linux-gnu ci/cross
	cross build \
		--release \
		--no-default-features \
		--target x86_64-unknown-linux-gnu \
		--features target-x86_64-unknown-linux-gnu

image: build_x86_64-unknown-linux-musl
	cp target/x86_64-unknown-linux-musl/release/vertex distribution/docker
	cd distribution/docker && strip vertex && docker build -t vertex:${VERSION}-alpine .

cross:
	cross build --target x86_64-unknown-linux-musl --no-default-features --features target-x86_64-unknown-linux-musl

# profile when bench
# cargo bench --bench hwmon_gather -- --profile-time=30