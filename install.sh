#!/bin/sh
# Install EVE Settings Manager for the current user (no root needed).
#
#   ./install.sh                 install (uses the prebuilt binary if present, else builds with cargo)
#   ./install.sh --no-matugen    install without touching your matugen config
#   ./install.sh --uninstall     remove the app (keeps your notes, settings and backups)
set -e
cd "$(dirname "$0")"

APP=eve-settings-manager
BIN="$HOME/.local/bin"
APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICONS="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/128x128/apps"
CONF="${XDG_CONFIG_HOME:-$HOME/.config}"
MG_DIR="$CONF/matugen"
MG_CONF="$MG_DIR/config.toml"
MG_TPL="$MG_DIR/templates/$APP.json"
OUT="$CONF/$APP/colors.json"

MATUGEN=1
for arg in "$@"; do
  case "$arg" in
    --no-matugen) MATUGEN=0 ;;
    --uninstall)
      rm -f "$BIN/$APP" "$APPS/$APP.desktop" "$ICONS/$APP.png" "$MG_TPL"
      [ -f "$MG_CONF" ] && sed -i "/^\[templates.$APP\]/,/^output_path/d" "$MG_CONF"
      echo "Uninstalled. Your settings, notes and backups were kept in:"
      echo "  $CONF/$APP"
      echo "  ${XDG_DATA_HOME:-$HOME/.local/share}/$APP"
      exit 0 ;;
    -h|--help) sed -n '2,6p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "Unknown option: $arg" >&2; exit 2 ;;
  esac
done

# 1. Binary: prebuilt (release tarball) or build from source.
if [ -x "./$APP" ]; then
  SRC="./$APP"
  echo "Using prebuilt binary."
else
  command -v cargo >/dev/null || { echo "cargo not found. Install Rust (e.g. 'sudo pacman -S rust' or https://rustup.rs) or use a release tarball." >&2; exit 1; }
  cargo build --release --locked
  SRC="target/release/$APP"
fi
install -Dm755 "$SRC" "$BIN/$APP"
# Use the full path: launchers started by the compositor often don't have ~/.local/bin on PATH.
mkdir -p "$APPS"
sed "s|^Exec=.*|Exec=\"$BIN/$APP\"|" assets/$APP.desktop > "$APPS/$APP.desktop"
chmod 644 "$APPS/$APP.desktop"
install -Dm644 assets/$APP.png "$ICONS/$APP.png"
command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q "${ICONS%/128x128/apps}" 2>/dev/null || true

# 2. Optional matugen integration.
if [ "$MATUGEN" = 1 ] && command -v matugen >/dev/null; then
  install -Dm644 matugen/$APP.json "$MG_TPL"
  mkdir -p "$MG_DIR"; touch "$MG_CONF"
  if ! grep -q "^\[templates.$APP\]" "$MG_CONF"; then
    cp "$MG_CONF" "$MG_CONF.bak-$APP-$(date +%s)"
    printf '\n[templates.%s]\ninput_path = "%s"\noutput_path = "%s"\n' \
      "$APP" "$MG_TPL" "$OUT" >> "$MG_CONF"
    echo "Added the $APP template to $MG_CONF (backup saved next to it)."
  fi
  # Generate colours right away if we can find the current wallpaper.
  WP="$(readlink -f "$HOME/.cache/current-wallpaper" 2>/dev/null || true)"
  if [ -f "$WP" ]; then
    TMP="$(mktemp)"
    printf '[config]\n[templates.esm]\ninput_path = "%s"\noutput_path = "%s"\n' "$MG_TPL" "$OUT" > "$TMP"
    mkdir -p "$(dirname "$OUT")"
    matugen image "$WP" -m dark -c "$TMP" -q && echo "Generated colours from your current wallpaper."
    rm -f "$TMP"
  else
    echo "Colours will appear the next time you run matugen (e.g. 'matugen image <wallpaper>')."
  fi
elif [ "$MATUGEN" = 1 ]; then
  echo "matugen not found: the app will use the EVE faction / custom themes (Appearance menu)."
fi

case ":$PATH:" in
  *":$BIN:"*) ;;
  *) echo "Note: $BIN is not on your PATH. Add it, or launch from your app menu." ;;
esac
echo "Installed. Launch 'EVE Settings Manager' from your app launcher or run: $APP"
