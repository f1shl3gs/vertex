#! /usr/bin/env bash

set -e -o verbose

rustup show # causes installation of version from rust-toolchain.toml
rustup default "$(rustup show active-toolchain | awk '{print $1;}')"

# Setup cargo-cross
if [[ "$(cross --version | grep cross)" != "cross 0.2.5" ]]; then
  rustup run stable cargo install cross --version 0.2.5 --force
fi

# Make sure our release build settings are present.
#
# We want to ensure we're building using "full" release capabilities when possible, which
# means full LTO and a single codegen unit.  This maximizes performance of the resulting
# code, but increases compilation time.  We only set this if we're in CI _and_ we haven't
# been instructed to use the debug profile (via PROFILE environment variable).
if [[ "${CI-}" == "true" && "${PROFILE-}" != "debug" ]]; then
  {
    echo "CARGO_PROFILE_RELEASE_LTO=fat";
    echo "CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1";
    echo "CARGO_PROFILE_RELEASE_DEBUG=false";
  } >> "${GITHUB_ENV}"
fi

bash ci/install_protoc.sh

# Install mold, because the system linker wastes a bunch of time.
#
# Notably, we don't install/configure it when we're going to do anything with `cross`, as `cross` takes the Cargo
# configuration from the host system and ships it over...  which isn't good when we're overriding the `rustc-wrapper`
# and all of that.
MOLD_VERSION=2.3.3
CARGO_DIR="${HOME}/.cargo"
if [[ "$(${CARGO_DIR}/bin/mold -v | awk '{print $2}')" != "$MOLD_VERSION" ]]; then
  # We explicitly put `mold-wrapper.so` right beside `mold` itself because it's hard-coded to look in the same directory
  # first when trying to load the shared object, so we can dodge having to care about the "right" lib folder to put it in.
  TEMP=$(mktemp -d)
  MOLD_TARGET=mold-${MOLD_VERSION}-$(uname -m)-linux
  curl -fsSL "https://github.com/rui314/mold/releases/download/v${MOLD_VERSION}/${MOLD_TARGET}.tar.gz" \
      --output "$TEMP/${MOLD_TARGET}.tar.gz"
  tar \
      -xvf "${TEMP}/${MOLD_TARGET}.tar.gz" \
      -C "${TEMP}"
  cp "${TEMP}/${MOLD_TARGET}/bin/mold" "${CARGO_DIR}/bin/mold"
  cp "${TEMP}/${MOLD_TARGET}/lib/mold/mold-wrapper.so" "${CARGO_DIR}/bin/mold-wrapper.so"
  rm -rf "$TEMP"
fi

# Now configure Cargo to use our rustc wrapper script.
# echo "export PATH=\"$HOME/.cargo/bin:\$PATH\"" >> "${HOME}/.bash_profile"
cat <<EOF >>"${CARGO_DIR}/config.toml"
[target.x86_64-unknown-linux-gnu]
linker="clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
EOF
