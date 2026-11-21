# EVE Settings Manager (native Linux)

A Rust rewrite of EVE Settings Manager 2.0.6 (originally Electron, Windows-only).
Built with egui and styled after Material 3 (Material You). It runs natively on
Wayland/Hyprland, with no webview.

## Features
- Finds `CCP/EVE` inside Proton, Lutris/umu, Heroic, Bottles and Wine prefixes,
  including Steam library folders. You can also pick the folder by hand.
- Servers (Tranquility, Serenity, …) with ESI online status.
- Profiles (`settings_*`): create, rename, duplicate, delete.
- Characters and accounts with names looked up through ESI, plus account notes.
- Sync one character's or account's settings onto others.
- Backups: whole-profile snapshots and single files, with restore and delete.
- Notices settings changes made by the game client while the app is open.

## Theming
Open the Appearance menu (palette icon) to choose:
- **Matugen**: follows your wallpaper and updates live. `install.sh` adds a
  template to `~/.config/matugen/config.toml` that writes
  `~/.config/eve-settings-manager/colors.json`.
- **EVE factions**: Caldari, Amarr, Gallente, Minmatar. Caldari is the default
  when matugen isn't installed.
- **Custom**: pick a seed colour and a palette style (Tonal spot, Vibrant,
  Expressive, Fidelity, Neutral, Monochrome).

Palettes come from `material-colors`, the same Material You engine matugen uses.
The brightness setting (Auto / Light / Dark) applies to every theme.

## Install
```sh
./install.sh             # builds release, installs to ~/.local/bin + desktop entry
./install.sh --uninstall
```
Hyprland window rules can match `class:^(eve-settings-manager)$`.

## Data
- Config and notes: `~/.config/eve-settings-manager/config.json`
- Backups: `~/.local/share/eve-settings-manager/backups/`

## Credits
This is a port of **EVE Settings Manager** by **Nick Ning**, released under the
MIT License (see `LICENSE`). The features, workflow and UI text follow the original 2.0.6
release, and the app icon (`assets/eve-settings-manager.png`) comes from it.
Thanks to Nick for the original app.

EVE Online and all related names are trademarks of CCP hf. This project is not
affiliated with or endorsed by CCP.
