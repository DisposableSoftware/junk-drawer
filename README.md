<p align="center">
  <img src="docs/icon.svg" width="96" alt="Junk Drawer icon">
</p>

<h1 align="center">Junk Drawer</h1>

<p align="center">A small GTK 4 file manager for Hyprland, in Catppuccin colours.</p>

> [!WARNING]
> **This is vibe-coded slop. Don't use it.**
>
> Junk Drawer was written by an AI coding agent, with a human steering and clicking
> "approve". Nobody has reviewed the code line by line, and it has had almost no real use.
> It moves, copies and deletes your files, and it may well do that wrong.
> It's published as an experiment, not as software for anyone to rely on: if you want a
> file manager, use Nautilus, Thunar or anything else with actual maintainers.
> No support, no promises; issues and pull requests may never be read.

![Junk Drawer's icon view, with thumbnails and the sidebar](docs/screenshots/icons.png)

Junk Drawer (`junk`) is a minimal file manager for tiling Wayland setups. It has no
toolbar, status bar or libadwaita, and every action can be bound to keys of your choice.

## Features

- **Two views:** an icon grid with thumbnails, and a details list with Size, Modified
  and Type columns. Sort by any column, with names sorted naturally (`file2` before `file10`).
- **Sidebar:** Recent, Home, your XDG folders, bookmarks (shared with Nautilus and the
  GTK file chooser), and removable drives you can mount, unmount and eject.
- **File operations:** copy, move, drag and drop between apps, cut and paste
  that works with Nautilus and Thunar, trash, and a name-clash dialog (Replace, Merge,
  Skip, Rename) with a progress window for long transfers.
- **Catppuccin theming:** all four flavors and all 14 accents, a transparent window
  background, or `flavor = gtk` to follow your GTK theme (Stylix included).
- **Keyboard first:** every binding can be changed in `config.ini`; press <kbd>?</kbd> to see them all.
- **Freedesktop integration:** thumbnails shared with other apps, the XDG trash,
  `org.freedesktop.FileManager1` for "Show in folder" in browsers, and the default
  apps from `mimeapps.list`.

| | |
|---|---|
| ![Details view](docs/screenshots/details.png) | ![Keyboard shortcuts overlay](docs/screenshots/help.png) |
| ![Context menu](docs/screenshots/menu.png) | ![Catppuccin Latte with the blue accent](docs/screenshots/latte.png) |

## Install

Read the warning at the top first.

### Nix

Try it without installing:

```sh
nix run github:DisposableSoftware/junk-drawer
```

As a flake input, with the NixOS module:

```nix
{
  inputs.junk-drawer.url = "github:DisposableSoftware/junk-drawer";

  # in your NixOS configuration
  imports = [ inputs.junk-drawer.nixosModules.default ];
  programs.junk-drawer = {
    enable = true;
    defaultFileManager = true; # open folders with Junk Drawer
  };
}
```

The NixOS module also turns on `services.gvfs` (trash, Recent, network locations)
and `services.udisks2` (removable drives).

Or with home-manager, which can also write your settings:

```nix
{
  imports = [ inputs.junk-drawer.homeManagerModules.default ];
  programs.junk-drawer = {
    enable = true;
    defaultFileManager = true;
    settings = {
      appearance = { flavor = "mocha"; accent = "mauve"; opacity = 0.85; };
      view.default = "icons";
      keybindings.go-up = [ "<Alt>Up" "BackSpace" ];
    };
  };
}
```

With either module, `fileManager1` (on whenever `defaultFileManager` is) lets D-Bus start
Junk Drawer for "Show in folder" requests when it isn't running. There is also an overlay,
`overlays.default`, which adds `pkgs.junk-drawer`.

### From source

You need Rust 1.92 or newer, GTK 4.20 or newer, and `pkg-config`. Then:

```sh
cargo build --release
install -Dm755 target/release/junk ~/.local/bin/junk
install -Dm644 data/io.github.DisposableSoftware.JunkDrawer.desktop -t ~/.local/share/applications
install -Dm644 data/io.github.DisposableSoftware.JunkDrawer.svg -t ~/.local/share/icons/hicolor/scalable/apps
install -Dm644 data/junk.1 -t ~/.local/share/man/man1
```

For the trash, Recent and network locations, install `gvfs`; for removable drives,
`udisks2`. Thumbnails for videos, PDFs and other files come from any installed
thumbnailer, such as `ffmpegthumbnailer` or `evince`.

## Usage

```sh
junk                 # a window on your home folder
junk ~/Downloads /mnt sftp://host/srv   # one window for each path or URI
junk --help
man junk             # every binding, every setting, and how thumbnails work
```

Junk Drawer runs as a single instance, so new windows open in the running process.
Every window, dialogs included, has the Wayland app ID
`io.github.DisposableSoftware.JunkDrawer`, which you can match in Hyprland window rules.

### Mouse

Double-click opens an item, and right-click opens the context menu. The back and forward
thumb buttons navigate history, and <kbd>Ctrl</kbd>+scroll zooms. Drag files to move them
on the same filesystem or copy them to another; hold <kbd>Ctrl</kbd> to copy or
<kbd>Shift</kbd> to move.

### Keyboard

These are the defaults. Press <kbd>?</kbd> in any window to see all of them, with your changes.

| Action | Keys |
|---|---|
| Open | <kbd>Return</kbd>, <kbd>Space</kbd> |
| Go up / back / forward | <kbd>BackSpace</kbd>, <kbd>Alt</kbd>+<kbd>↑</kbd> / <kbd>Alt</kbd>+<kbd>←</kbd> / <kbd>Alt</kbd>+<kbd>→</kbd> |
| Home folder | <kbd>Alt</kbd>+<kbd>Home</kbd> |
| Type a location (Tab completes) | <kbd>Ctrl</kbd>+<kbd>L</kbd> |
| Icon / details view | <kbd>Ctrl</kbd>+<kbd>1</kbd> / <kbd>Ctrl</kbd>+<kbd>2</kbd> |
| Show hidden files | <kbd>Ctrl</kbd>+<kbd>H</kbd> |
| Toggle the sidebar | <kbd>F9</kbd> |
| Bookmark this folder | <kbd>Ctrl</kbd>+<kbd>D</kbd> |
| Copy / cut / paste | <kbd>Ctrl</kbd>+<kbd>C</kbd> / <kbd>Ctrl</kbd>+<kbd>X</kbd> / <kbd>Ctrl</kbd>+<kbd>V</kbd> |
| Rename | <kbd>F2</kbd> |
| New folder | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>N</kbd> |
| Move to trash / delete permanently | <kbd>Delete</kbd> / <kbd>Shift</kbd>+<kbd>Delete</kbd> |
| Properties | <kbd>Ctrl</kbd>+<kbd>I</kbd> |
| Context menu | <kbd>Menu</kbd>, <kbd>Shift</kbd>+<kbd>F10</kbd> |
| Zoom in / out / reset | <kbd>Ctrl</kbd>+<kbd>+</kbd> / <kbd>Ctrl</kbd>+<kbd>-</kbd> / <kbd>Ctrl</kbd>+<kbd>0</kbd> |
| New window / close window | <kbd>Ctrl</kbd>+<kbd>N</kbd> / <kbd>Ctrl</kbd>+<kbd>W</kbd>, <kbd>Q</kbd> |

## Configuration

Settings live in `~/.config/junk-drawer/config.ini`. Every key is optional, and changes
apply when Junk Drawer restarts. [`data/config.example.ini`](data/config.example.ini)
lists every setting and binding with its default.

```ini
[appearance]
# latte, frappe, macchiato or mocha, or gtk to use your GTK theme's colours
flavor = mocha
accent = mauve
opacity = 0.85

[view]
default = icons
show-sidebar = true
show-thumbnails = true

[keybindings]
# GTK accelerator syntax; separate keys with ';', leave empty to unbind
go-up = <Alt>Up;BackSpace;<Ctrl>u
trash =
```

For extra styling, put GTK CSS in `~/.config/junk-drawer/style.css`. The palette's colour
names (`@base`, `@mauve`, `@accent`, …) are available there, and Junk Drawer's own rules
are all under `window.junk`:

```css
window.junk .sidebar { background: alpha(@crust, 0.6); }
```

## Development

What Junk Drawer does is written down in [`spec/`](spec/README.md): every behaviour is
a numbered requirement, and the code follows it. `cargo test` checks the keybinding and
settings tables in the spec against the code, the example config and the man page.

```sh
nix develop                 # cargo, clippy, rustfmt, rust-analyzer, GTK 4
cargo run
cargo clippy --all-targets -- -D warnings
cargo test
nix build                   # ./result/bin/junk
```

The scripts in [`scripts/manual/`](scripts/manual) drive the app on Hyprland to check
what unit tests can't, such as focus, menus and screenshots.

## License

MIT, see [LICENSE](LICENSE).
