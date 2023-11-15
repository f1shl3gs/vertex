#!/bin/sh

set -e -o verbose

apt-get update
apt-get install -y apt-transport-https wget unzip
apt clean

# Setup protoc
#
# prost-build need protoc to be installed
PROTOC_VERSION=3.20.1

curl -L "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip" -o protoc-${PROTOC_VERSION}-linux-x86_64.zip
unzip protoc-${PROTOC_VERSION}-linux-x86_64.zip
cp bin/protoc /usr/local/bin
cp -r include/google /usr/local/include/
rm protoc-${PROTOC_VERSION}-linux-x86_64.zip

# Setup mold
#
# We explicitly put `mold-wrapper.so` right beside `mold` itself because it's hard-coded to look in the same directory
# first when trying to load the shared object, so we can dodge having to care about the "right" lib folder to put it in.
TEMP=$(mktemp -d)
MOLD_VERSION=2.3.3
MOLD_TARGET=mold-${MOLD_VERSION}-$(uname -m)-linux
curl -fsSL "https://github.com/rui314/mold/releases/download/v${MOLD_VERSION}/${MOLD_TARGET}.tar.gz" \
    --output "$TEMP/${MOLD_TARGET}.tar.gz"
tar \
    -xvf "${TEMP}/${MOLD_TARGET}.tar.gz" \
    -C "${TEMP}"
cp "${TEMP}/${MOLD_TARGET}/bin/mold" /usr/bin/mold
cp "${TEMP}/${MOLD_TARGET}/lib/mold/mold-wrapper.so" /usr/bin/mold-wrapper.so
rm -rf "$TEMP"

# Now configure Cargo to use our rustc wrapper script.
mkdir "${HOME}/.cargo"
cat <<EOF >>"${HOME}/.cargo/config.toml"
[target.x86_64-unknown-linux-gnu]
linker="clang"
rustflags = ["-C", "link-arg=-fuse-ld=/usr/bin/mold"]
EOF
