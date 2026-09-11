# Docky.rs

<img width="784" height="88" alt="image" src="https://github.com/user-attachments/assets/7f72d11a-18c3-4a05-a313-33dea8de0d9f" />


A dock/bar for Hyprland, written in Rust. Pinned apps, Apps search, Clipboard manager, Screenshot tool, a wallpaper picker, a widget bar (clock, workspaces, media, CPU/RAM, tray), volume and brightness OSDs, and notifications.

The built binary is around 13MB and it sits around 10-65mb of RAM while running.


## Requirements

* Hyprland
* Arch Linux or Void Linux

## Install

```
git clone https://github.com/Volcha-Z/Docky.rs.git
cd Docky.rs
./install.sh
```

`install.sh` detects your distro, installs everything the dock needs, and builds it with `cargo build --release`. The finished binary ends up at `target/release/dockyrs`.

## Config

Settings live at `~/.config/dockyrs/config.json` and are created automatically the first time you run the dock. There is nothing you need to set up by hand.

## Wallpaper based theming (optional)

If you turn on `accent_from_wallpaper` in the config, picking a new wallpaper through the dock will also regenerate colors for Dolphin, kitty, and a Powerlevel10k using matugen.

## License

MIT, see LICENSE.

## Keybinds

| Key | Action |
| --- | --- |
| `SUPER` + `D` | App search |
| `SUPER` + `W` | Wallpaper picker |
| `SUPER` + `V` | Clipboard manager |
| `SUPER` + `,` | Dock menu |
| `SUPER` + `R` | Relaunch dock |
| `SUPER` + `CTRL` + `SHIFT` + `1` | Test notification |
| `F8` | Screenshot region |
| `F9` | Screenshot full |
| `F10` | Toggle screen recording |

## Wallpaper selector 

Place your wallpapers here ~/Pictures/wallpapers u might need to create the folders yourself!