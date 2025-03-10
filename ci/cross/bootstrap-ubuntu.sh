#!/bin/sh

set -e -o verbose

apt-get update
apt-get install -y apt-transport-https wget unzip
apt clean

# print as verion
as -version

# Setup protoc
#
# prost-build need protoc to be installed
PROTOC_VERSION=3.20.1

curl -L "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip" -o protoc-${PROTOC_VERSION}-linux-x86_64.zip
unzip protoc-${PROTOC_VERSION}-linux-x86_64.zip
cp bin/protoc /usr/local/bin
cp -r include/google /usr/local/include/
rm protoc-${PROTOC_VERSION}-linux-x86_64.zip
