#!/bin/sh

#
# Release build script for reproducible builds
#
# Usage: docker run --rm -v "$(pwd)":/src -w /src rust:1.38.0-alpine "./build_release.sh"
#

apk add --no-cache jq

rustup target add x86_64-unknown-linux-musl

cargo build --release --target x86_64-unknown-linux-musl

OUTPUT_DIR=target/x86_64-unknown-linux-musl/release
OUTPUT_NAME=run-all

# extract version from Cargo.toml
VERSION=$(cargo metadata --no-deps --format-version 1 | jq -r .packages[0].version)

tar -czv -f "run-all_${VERSION}_bin.tar.gz" -C "${OUTPUT_DIR}" "${OUTPUT_NAME}"

