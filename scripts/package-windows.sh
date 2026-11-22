#!/bin/sh
# Cross-compile the Windows build from Linux:
#   dist/eve-settings-manager-<version>-x86_64-windows.zip (+ .sha256)
# Needs: rustup target add x86_64-pc-windows-gnu, and mingw-w64-gcc (Arch) / gcc-mingw-w64 (Debian).
set -e
cd "$(dirname "$0")/.."
APP=eve-settings-manager
TARGET=x86_64-pc-windows-gnu
VER=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
NAME="$APP-$VER-x86_64-windows"
# Prefer rustup's cargo/rustc (distro Rust packages usually lack the Windows std).
if command -v rustup >/dev/null; then
  PATH="$(dirname "$(rustup which rustc)"):$PATH"
fi
CARGO_DIR="${CARGO_HOME:-$HOME/.cargo}"
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$CARGO_DIR=/cargo --remap-path-prefix=$PWD=/build --remap-path-prefix=$HOME=/home/builder"
cargo build --release --locked --target "$TARGET" --target-dir target/dist-windows
rm -rf "dist/$NAME" && mkdir -p "dist/$NAME"
cp "target/dist-windows/$TARGET/release/$APP.exe" README.md LICENSE CHANGELOG.md "dist/$NAME/"
x86_64-w64-mingw32-strip "dist/$NAME/$APP.exe" 2>/dev/null || true
if strings "dist/$NAME/$APP.exe" | grep -q -e "$HOME" -e "$(id -un)"; then
  echo "error: exe still contains the builder's home path or user name" >&2; exit 1
fi
rm -f "dist/$NAME.zip"
# -X: no uid/gid or extra timestamps in the archive.
(cd dist && zip -qrX "$NAME.zip" "$NAME" && sha256sum "$NAME.zip" > "$NAME.zip.sha256")
echo "dist/$NAME.zip"
