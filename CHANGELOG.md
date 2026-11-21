# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [2.0.6] - 2026-11-21

First native Linux release. It's a full rewrite in Rust of the Windows/Electron
EVE Settings Manager 2.0.6 by Nick Ning.

### Added
- Native Wayland/X11 app built with egui (no Electron/webview), with a ~24 MB
  single binary.
- Automatic discovery of `CCP/EVE` in Steam/Proton (including extra Steam
  libraries and Flatpak Steam), Lutris/umu, Heroic, Bottles and plain Wine
  prefixes.
- Material 3 (Material You) interface: navigation drawer, which switches to a
  navigation rail on narrow or tiled windows.
- Theming:
  - matugen integration with live reload when the wallpaper changes,
  - four EVE faction themes (Caldari, Amarr, Gallente, Minmatar),
  - custom themes from any seed colour, in six palette styles,
  - Auto / Light / Dark brightness.
- Single-file backups now keep their history: each backup is timestamped
  instead of overwriting the previous one.
- Settings files changed by the game client while the app is open appear
  automatically.
- `install.sh` per-user installer with desktop entry, icon and optional
  matugen template.

### Carried over from the original
- Servers with ESI online status, profiles (create / rename / duplicate /
  delete), character and account settings, sync to other characters or
  accounts, account notes, ESI character-name lookup, and whole-profile backups
  with restore.

### Not carried over
- UI translations (English only for now).

[2.0.6]: https://github.com/tylort69/eve-settings-manager/releases/tag/v2.0.6
