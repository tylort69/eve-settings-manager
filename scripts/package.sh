#!/bin/sh
# Build a release tarball: dist/eve-settings-manager-<version>-x86_64-linux.tar.gz (+ .sha256)
set -e
cd "$(dirname "$0")/.."
APP=eve-settings-manager
VER=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
NAME="$APP-$VER-$(uname -m)-linux"
cargo build --release --locked
rm -rf "dist/$NAME" && mkdir -p "dist/$NAME"
cp target/release/$APP install.sh README.md LICENSE CHANGELOG.md "dist/$NAME/"
cp -r assets matugen "dist/$NAME/"
tar -C dist -czf "dist/$NAME.tar.gz" "$NAME"
(cd dist && sha256sum "$NAME.tar.gz" > "$NAME.tar.gz.sha256")
echo "dist/$NAME.tar.gz"
