build:
	cargo build --release

build-musl:
	cargo build --release --target=x86_64-unknown-linux-musl