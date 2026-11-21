#!/bin/sh
# Build a release tarball: dist/eve-settings-manager-<version>-x86_64-linux.tar.gz (+ .sha256)
set -e
cd "$(dirname "$0")/.."
APP=eve-settings-manager
VER=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
NAME="$APP-$VER-$(uname -m)-linux"
# Don't embed the builder's home/project paths (panic locations) in the binary.
CARGO_DIR="${CARGO_HOME:-$HOME/.cargo}"
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$CARGO_DIR=/cargo --remap-path-prefix=$PWD=/build --remap-path-prefix=$HOME=/home/builder"
cargo build --release --locked --target-dir target/dist
rm -rf "dist/$NAME" && mkdir -p "dist/$NAME"
cp target/dist/release/$APP install.sh README.md LICENSE CHANGELOG.md "dist/$NAME/"
cp -r assets matugen "dist/$NAME/"
tar --owner=0 --group=0 --numeric-owner -C dist -czf "dist/$NAME.tar.gz" "$NAME"
(cd dist && sha256sum "$NAME.tar.gz" > "$NAME.tar.gz.sha256")
if strings "dist/$NAME/$APP" | grep -q "$HOME"; then
  echo "error: binary still contains $HOME" >&2; exit 1
fi
echo "dist/$NAME.tar.gz"
