build:
	cargo build --release

build-musl:
	cargo build --release --target=x86_64-unknown-linux-musl

update_testdata:
	./ttar  -c -f testdata.ttar testdata

.PHONY: testdata
testdata:
	rm -rf testdata
	./ttar -x -f testdata.ttar

build-timing:
	cargo +nightly build -p vertex --bin vertex -Z timings --release

bloat:
	cargo bloat --release --crates

lines:
	@./scripts/lines.sh




# profile when bench
# cargo bench --bench hwmon_gather -- --profile-time=30