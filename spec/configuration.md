# Configuration

## Requirements

| ID | Requirement | Status | Verified by |
|----|-------------|--------|-------------|
| `CONF-1` | `[appearance] flavor`: latte, frappe, macchiato or mocha, or `gtk` to follow the GTK theme (`LOOK-15`). | done | `spec_tests`, `theme::tests`, `scripts/manual/gtk-theme.sh` |
| `CONF-2` | `[appearance] accent`: any Catppuccin accent name. It is ignored when `flavor` is `gtk`, which uses the theme's accent (`LOOK-15`). | done | `spec_tests`, `theme::tests::gtk_flavor_uses_the_theme_accent`, `scripts/manual/gtk-theme.sh` (`accent = red` is ignored) |
| `CONF-3` | `[appearance] opacity`: the window background alpha, clamped to 0.0–1.0. | done | `spec_tests` |
| `CONF-4` | `[view] default`: `icons` or `details`. Any other value is ignored. | done | `spec_tests` |
| `CONF-5` | `[view] show-hidden`: show hidden and backup files in new windows. | done | `config::tests` |
| `CONF-6` | `[view] icon-size`: grid icon size in px, clamped to 16–512. | done | `spec_tests` |
| `CONF-7` | `[view] list-icon-size`: details icon size in px, clamped to 16–128. | done | `spec_tests` |
| `CONF-8` | Keybindings are read from `$XDG_CONFIG_HOME/junk-drawer/keybindings.ini`, not `config.ini`: see `KEY-2`. | planned | `config::tests` |
| `CONF-9` | Settings are read from `$XDG_CONFIG_HOME/junk-drawer/config.ini` (GLib key-file / INI syntax). A missing file is not an error. A file that can't be read or parsed is warned about, and the defaults are used. Every key is optional. | done | manual (scripts write `junk-drawer/config.ini` in a temp `XDG_CONFIG_HOME`: `nav-focus.sh`, `thumbnails.sh`, `sidebar.sh`, `key-help.sh`) |
| `CONF-10` | Extra GTK CSS is read from `$XDG_CONFIG_HOME/junk-drawer/style.css` (see `LOOK-6`). | done | manual (`junk --help` names the path; code review) |
| `CONF-11` | Changes apply when the app is restarted. | done | — |
| `CONF-12` | `data/config.example.ini` lists every setting with its default, and `data/keybindings.example.ini` every binding with its default keys. Both are installed to `share/doc/junk-drawer/`. | planned | `spec_tests::example_config_matches_spec` |
| `CONF-13` | `[view] show-path`: show the path bar and its up button (`VIEW-30`, `VIEW-36`). When `false`, both are hidden and take no space. | done | `config::tests::parses_keybindings`, `spec_tests`, `scripts/manual/path-bar.sh` (temp `XDG_CONFIG_HOME`) |
| `CONF-14` | `[view] grid-caption`: the icon-view caption (`VIEW-33`): `size`, `modified`, `type`, `permissions` or `none`. Any other value is ignored. | done | `config::tests::ignores_unknown_grid_caption`, `config::tests::parses_keybindings`, `spec_tests` |
| `CONF-15` | `[view] show-sidebar`: whether windows start with the sidebar shown (`NAV-41`). | done | `spec_tests`, `scripts/manual/sidebar.sh` (temp `XDG_CONFIG_HOME`) |
| `CONF-16` | `[view] show-thumbnails`: show thumbnails in place of icons (`VIEW-37`). | done | `spec_tests`, `scripts/manual/thumbnails.sh` (temp `XDG_CONFIG_HOME`) |
| `CONF-17` | When Junk Drawer starts and `config.ini` or `keybindings.ini` doesn't exist in `$XDG_CONFIG_HOME/junk-drawer/`, it creates the missing file (and the folder) from the example (`CONF-12`), with every setting or binding listed at its default but commented out, so the file changes nothing until a line is uncommented. A file that exists is never written to, whether it is a regular file, a symlink (as with dotfiles or home-manager) or a dangling symlink. If a file can't be created, that is warned about once on stderr and Junk Drawer starts normally. | planned | `config::tests` (a temp folder: missing files are created and load as the defaults; uncommenting a line takes effect; existing files and symlinks are left byte-for-byte unchanged; an unwritable folder only warns), manual (first run with an empty temp `XDG_CONFIG_HOME`) |

## Defaults

<!-- spec:config:begin -->
| Section | Key | Default |
|---------|-----|---------|
| `appearance` | `flavor` | `mocha` |
| `appearance` | `accent` | `mauve` |
| `appearance` | `opacity` | `0.85` |
| `view` | `default` | `icons` |
| `view` | `show-hidden` | `false` |
| `view` | `icon-size` | `96` |
| `view` | `list-icon-size` | `32` |
| `view` | `show-path` | `true` |
| `view` | `grid-caption` | `size` |
| `view` | `show-sidebar` | `true` |
| `view` | `show-thumbnails` | `true` |
<!-- spec:config:end -->

The defaults for `[keybindings]` are in [keybindings.md](keybindings.md).

## Implementation notes

- To add a key, edit the defaults table, `Config`, `spec_tests::default_value` and `CONFIG_KEY_COUNT`, `data/config.example.ini` and the man page `data/junk.1` (`INT-32`). The spec tests fail until they agree.
