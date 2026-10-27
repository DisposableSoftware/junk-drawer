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
| `CONF-8` | Keybindings are read from `$XDG_CONFIG_HOME/junk-drawer/keybindings.ini`, not `config.ini`: see `KEY-2`. | done | `config::tests::reads_settings_and_keybindings_from_their_files` |
| `CONF-9` | Settings are read from `$XDG_CONFIG_HOME/junk-drawer/config.ini` (GLib key-file / INI syntax). A missing file is not an error. A file that can't be read or parsed is warned about, and the defaults are used. Every key is optional. | done | manual (scripts write `junk-drawer/config.ini` in a temp `XDG_CONFIG_HOME`: `nav-focus.sh`, `thumbnails.sh`, `sidebar.sh`, `key-help.sh`) |
| `CONF-10` | Extra GTK CSS is read from `$XDG_CONFIG_HOME/junk-drawer/style.css` (see `LOOK-6`). | done | manual (`junk --help` names the path; code review) |
| `CONF-11` | Changes to `config.ini`, `keybindings.ini` and `style.css` apply when the app is restarted. | done | — |
| `CONF-12` | `data/config.example.ini` lists every setting with its default, and `data/keybindings.example.ini` every binding with its default keys. Both are installed to `share/doc/junk-drawer/`. | done | `spec_tests::example_config_matches_spec` |
| `CONF-13` | `[view] show-path`: show the path bar and its up button (`VIEW-30`, `VIEW-36`). When `false`, both are hidden and take no space. | done | `config::tests::reads_settings_and_keybindings_from_their_files`, `spec_tests`, `scripts/manual/path-bar.sh` (temp `XDG_CONFIG_HOME`) |
| `CONF-14` | `[view] grid-caption`: the icon-view caption (`VIEW-33`): `size`, `modified`, `type`, `permissions` or `none`. Any other value is ignored. | done | `config::tests::ignores_unknown_grid_caption`, `config::tests::reads_settings_and_keybindings_from_their_files`, `spec_tests` |
| `CONF-15` | `[view] show-sidebar`: whether windows start with the sidebar shown (`NAV-41`). | done | `spec_tests`, `scripts/manual/sidebar.sh` (temp `XDG_CONFIG_HOME`) |
| `CONF-16` | `[view] show-thumbnails`: show thumbnails in place of icons (`VIEW-37`). | done | `spec_tests`, `scripts/manual/thumbnails.sh` (temp `XDG_CONFIG_HOME`) |
| `CONF-17` | When Junk Drawer starts and `config.ini` or `keybindings.ini` doesn't exist in `$XDG_CONFIG_HOME/junk-drawer/`, it creates the missing file (and the folder) from the example (`CONF-12`), with every setting or binding listed at its default but commented out, so the file changes nothing until a line is uncommented. A file that exists is never written to, whether it is a regular file, a symlink (as with dotfiles or home-manager) or a dangling symlink. If a file can't be created, that is warned about once on stderr and Junk Drawer starts normally. | done | `config::tests` (a temp folder: missing files are created and load as the defaults; uncommenting a line takes effect; existing files and symlinks are left byte-for-byte unchanged; an unwritable folder only warns), manual (first run with an empty temp `XDG_CONFIG_HOME` created both files, all values commented out) |
| `CONF-18` | Preferences changed while using the app are remembered in `$XDG_STATE_HOME/junk-drawer/state.ini` (default `~/.local/state/junk-drawer/state.ini`), a GLib key file. Only the app writes it, and it never writes `config.ini` or `keybindings.ini`. The file and its folder are created on the first save. A missing file is not an error. A file or value that can't be read or parsed is ignored (warned about once on stderr), and the config value is used. | done | `state::tests` (round trip, missing file, garbage file, invalid values), `scripts/manual/remember.sh` (no file before a change; `keybindings.ini` untouched) |
| `CONF-19` | What is remembered, and the config key it overrides: the view, icons or details (`[view] default`); the sort column and whether it is reversed (none); whether hidden and backup files are shown (`show-hidden`); whether the sidebar is shown (`show-sidebar`); whether the status bar is shown (`show-status-bar`); the icon-view and details-view zoom sizes (`icon-size`, `list-icon-size`). A new window uses each remembered value, and the config value for any that was never saved. `zoom-reset` (`VIEW-25`) goes back to the configured size and forgets the saved zoom of that view. Deleting `state.ini` restores the `config.ini` values. | done | `state::tests::remembered_values_override_config`, `scripts/manual/remember.sh` (each setting saved, `zoom-reset` forgets the zoom, restart restores the view and sort) |
| `CONF-20` | A remembered value is saved as soon as it changes, from any window (the last change wins), without blocking the window. Windows already open are not changed by another window's save. Saving the sort writes the column and direction together. A write failure is warned about once and the app goes on. | done | `scripts/manual/remember.sh` (each change is in `state.ini` at once, with two windows open); code review (open windows read the state only when created; GIO replaces the file atomically) |
| `CONF-21` | Window size: a new window opens at the size the last window had when it closed, in logical px (default 900×600, `KEY-10`). A window that is maximized, fullscreen or tiled when it closes does not save its size. Sizes under 400×300 are ignored. | dropped: on Hyprland 0.56 every window, floating ones included, reports tiled and maximized states, so a tile size can't be told from a floating one; set float sizes with a window rule | — |
| `CONF-22` | `[view] remember`: when `false`, nothing is remembered (`CONF-18`…`CONF-20`): `state.ini` is neither read nor written, so every window starts from the `config.ini` values, and a change made in a window lasts only for that window. An existing `state.ini` is left as it is, and is used again if the key goes back to `true`. | done | `state::tests::remember_false_neither_reads_nor_writes`, `config::tests`, `spec_tests`, `scripts/manual/remember-off.sh` (headless: a saved sort is used with `true` and ignored with `false`; nothing written) |
| `CONF-23` | `[view] show-status-bar`: whether windows start with the status bar shown (`NAV-44`, `VIEW-39`). | done | `spec_tests`, `scripts/manual/status-bar.sh` (temp `XDG_CONFIG_HOME`); headless: `show-status-bar=true` starts shown |
| `CONF-24` | In screenshot mode (`INT-33`) the settings are the defaults of the defaults table: `config.ini`, `keybindings.ini` and `style.css` are not read, nothing is created in the config folder (`CONF-17`), and `state.ini` is neither read nor written (as with `[view] remember = false`, `CONF-22`). | done | `scripts/manual/screenshot-mode.sh` (a temp `XDG_CONFIG_HOME` with `show-sidebar=false` and a `state.ini` still gives the default view, and neither file changes; no files are created); done headless: with a config (latte, no sidebar, details), a style.css and a state.ini in place, the window uses the defaults and no file changed |

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
| `view` | `show-status-bar` | `false` |
| `view` | `show-thumbnails` | `true` |
| `view` | `remember` | `true` |
<!-- spec:config:end -->

The defaults for `[keybindings]` are in [keybindings.md](keybindings.md).

## Implementation notes

- `state.ini` layout: `[view] mode`, `sort-column` (`name`, `size`, `modified`, `type`), `sort-reversed`, `show-hidden`, `show-sidebar`, `show-status-bar`, `icon-size`, `list-icon-size`. All keys optional. Write to a temp file and rename it, so a crash never leaves a half-written file. Document it in the man page under FILES (`INT-32`).
- To add a key, edit the defaults table, `Config`, `spec_tests::default_value` and `CONFIG_KEY_COUNT`, `data/config.example.ini` and the man page `data/junk.1` (`INT-32`). The spec tests fail until they agree.
