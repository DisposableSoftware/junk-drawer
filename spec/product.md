# Product

A lightweight, minimal Linux file manager for Hyprland. Display name **Junk Drawer**, binary `junk`, app ID `io.github.DisposableSoftware.JunkDrawer`.

## Requirements

| ID | Requirement | Status |
|----|-------------|--------|
| `PROD-1` | Lightweight and minimal: few dependencies, no libadwaita, fast startup. | done (ongoing constraint) |
| `PROD-2` | Follow a freedesktop/XDG spec wherever one exists for the behaviour (see [integration.md](integration.md)). | partial (ongoing constraint) |
| `PROD-3` | Mount, unmount and eject external devices. | done (see `INT-10`, `NAV-43`) |
| `PROD-4` | Multiple windows. | done (see `NAV-1`) |
| `PROD-5` | Drag and drop within the app and to/from other apps. | done (see `OPS-1`…`OPS-9`) |
| `PROD-6` | Looks good with a transparent background on Hyprland. | done (see `LOOK-2`) |
| `PROD-7` | Catppuccin themes: all four flavors and all 14 accents. | done (see `LOOK-1`) |
| `PROD-8` | Simple toolchain: `cargo` inside a Nix devshell, nothing else. | done |
| `PROD-9` | Easy NixOS integration. | done (see `INT-20`…`INT-24`) |
| `PROD-10` | Two views: an icon grid like Nautilus and a details view with columns. | done (see [views.md](views.md)) |
| `PROD-11` | Basic keybindings, which users can change in `~/.config/junk-drawer/`. | done (see [keybindings.md](keybindings.md)) |

## Non-goals

- No status bar or heavy chrome. The user runs Hyprland barless.
- No libadwaita. It fights custom theming and transparency.
- No plugin system, embedded terminal or built-in previewer.

## Stack constraints

- Rust and gtk4-rs (`gtk4` 0.11 with feature `gnome_49`; nixpkgs ships GTK 4.22 and GLib 2.88).
- Use GIO/GLib for anything they provide (XDG dirs, MIME, trash, mounts, clipboard formats) rather than reimplementing it or adding a crate.
- Ask before adding a crate.
- The binary is `junk`: short to type, and no common command or nixpkgs package uses the name. (The first name, `file`, would have shadowed `file(1)`.)
- File I/O is async through GIO. The main loop never blocks on I/O, except one cached local `query_info` while a drag hovers (see `OPS-3`).
