#!/usr/bin/env bash

# prost-build need protoc to be installed
PROTOC_VERSION=3.20.1

curl -L "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip" -o protoc-${PROTOC_VERSION}-linux-x86_64.zip
unzip protoc-${PROTOC_VERSION}-linux-x86_64.zip

cp bin/protoc /usr/local/bin

sudo mkdir -p /usr/local/include/google
sudo mv -r include/google /usr/local/include/
rm protoc-${PROTOC_VERSION}-linux-x86_64.zip
