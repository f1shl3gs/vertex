build:
	cargo build --release

build-musl:
	cargo build --release --target=x86_64-unknown-linux-musl

update_testdata:
	./ttar  -c -f testdata.ttar testdata

extract_testdata:
	./ttar -x -f testdata.ttar