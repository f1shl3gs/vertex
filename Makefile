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

image: build-musl
	@docker build -t vertex -f Dockerfile .

static:
	docker run --rm -it -v "/home/f1shl3gs/Workspaces/clion/vertex/docker/builder/cargo-config.toml:/opt/rust/cargo/config" -v "$$(pwd)":/home/rust/src musl-builder cargo build --release


# profile when bench
# cargo bench --bench hwmon_gather -- --profile-time=30