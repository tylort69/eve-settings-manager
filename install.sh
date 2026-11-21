#!/bin/sh
# Build and install EVE Settings Manager for the current user.
#   ./install.sh            build + install
#   ./install.sh --uninstall
set -e
cd "$(dirname "$0")"
BIN="$HOME/.local/bin"
APPS="$HOME/.local/share/applications"
ICONS="$HOME/.local/share/icons/hicolor/128x128/apps"
MG_DIR="$HOME/.config/matugen"
MG_CONF="$MG_DIR/config.toml"
MG_TPL="$MG_DIR/templates/eve-settings-manager.json"
OUT="$HOME/.config/eve-settings-manager/colors.json"

if [ "$1" = "--uninstall" ]; then
  rm -f "$BIN/eve-settings-manager" "$APPS/eve-settings-manager.desktop" "$ICONS/eve-settings-manager.png" "$MG_TPL"
  [ -f "$MG_CONF" ] && sed -i '/^\[templates.eve-settings-manager\]/,/^output_path/d' "$MG_CONF"
  echo "Uninstalled (your config, notes and backups in ~/.config and ~/.local/share/eve-settings-manager were kept)."
  exit 0
fi

cargo build --release
install -Dm755 target/release/eve-settings-manager "$BIN/eve-settings-manager"
install -Dm644 assets/eve-settings-manager.desktop "$APPS/eve-settings-manager.desktop"
install -Dm644 assets/eve-settings-manager.png "$ICONS/eve-settings-manager.png"
command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" 2>/dev/null || true

if command -v matugen >/dev/null; then
  install -Dm644 matugen/eve-settings-manager.json "$MG_TPL"
  mkdir -p "$MG_DIR"; touch "$MG_CONF"
  if ! grep -q '^\[templates.eve-settings-manager\]' "$MG_CONF"; then
    cp "$MG_CONF" "$MG_CONF.bak-eve-$(date +%s)"
    printf '\n[templates.eve-settings-manager]\ninput_path = "%s"\noutput_path = "%s"\n' \
      "~/.config/matugen/templates/eve-settings-manager.json" "~/.config/eve-settings-manager/colors.json" >> "$MG_CONF"
    echo "Added eve-settings-manager template to $MG_CONF"
  fi
  # Generate colours now from the current wallpaper, touching only this app's file.
  WP="$(readlink -f "$HOME/.cache/current-wallpaper" 2>/dev/null || true)"
  if [ -f "$WP" ]; then
    TMP="$(mktemp)"
    printf '[config]\n[templates.eve]\ninput_path = "%s"\noutput_path = "%s"\n' "$MG_TPL" "$OUT" > "$TMP"
    mkdir -p "$(dirname "$OUT")"
    matugen image "$WP" -m dark --source-color-index 0 -c "$TMP" -q && echo "Generated colours from $WP"
    rm -f "$TMP"
  else
    echo "Colours will be generated the next time you run matugen."
  fi
else
  echo "matugen not found: the app will use the EVE faction / custom themes (Appearance menu)."
fi
echo "Installed. Launch 'EVE Settings Manager' from your launcher or run: eve-settings-manager"
