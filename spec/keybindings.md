# Keybindings

## Requirements

| ID | Requirement | Status | Verified by |
|----|-------------|--------|-------------|
| `KEY-1` | Every user-facing action has a binding name, and its defaults are listed in the table below. | done | `spec_tests::keybindings_match_spec` |
| `KEY-2` | Users override a binding in the `[keybindings]` group of `$XDG_CONFIG_HOME/junk-drawer/keybindings.ini`, writing `name = accel;accel`. An empty value unbinds it. A binding that isn't listed keeps its defaults. A `[keybindings]` section left in `config.ini` is ignored, with one warning on stderr saying bindings now live in `keybindings.ini`. | planned | `config::tests`, manual (stderr) |
| `KEY-3` | Bindings without Ctrl, Alt or Super (letters, Space, Return, BackSpace, Delete, F-keys) work whenever the main window is focused, wherever focus is inside it. They are never triggered while a text field has focus (e.g. typing `q` in the rename box). | done | `scripts/manual/nav-focus.sh` |
| `KEY-4` | Bindings with Ctrl, Alt or Super are app-wide accelerators. | done | `keys::tests` |
| `KEY-5` | Unknown binding names and accelerators that can't be parsed are warned about once on stderr and skipped. They never crash the app. | done | manual (stderr) |
| `KEY-6` | Unmodified bindings take precedence over the list views' own handling of Space and Enter. | done | manual |
| `KEY-7` | Accelerator syntax is GTK's (`gtk_accelerator_parse`): `<Ctrl>`, `<Shift>`, `<Alt>`, `<Super>` plus a GDK key name. | done | `keys::tests` |
| `KEY-8` | The `show-help` binding (default `?`) shows a keybinding overlay over the file view. It lists every binding with a short description (e.g. "Go up") and its current keys: the defaults with the user's `[keybindings]` overrides applied (`KEY-2`). A binding with no keys (unbound in `[keybindings]`) is not listed. Keys are shown as readable labels (e.g. `Ctrl+N`, `Backspace`). | done | `keys::tests` (overrides and unbound bindings reach the overlay's rows), `scripts/manual/key-help.sh` (screenshot, default and overridden config) |
| `KEY-9` | While the overlay is shown, the next key press or mouse click dismisses it and does nothing else: the key is not passed to any binding or to the view. Pressing a modifier key alone (Shift, Ctrl, Alt, Super) does not dismiss it. After dismissal, keyboard focus is back on the file view (`NAV-30`). | done | `scripts/manual/key-help.sh` (`q` dismisses without closing the window, then `BackSpace` goes up) |
| `KEY-10` | The overlay shows all bindings at the default window size (900×600) without scrolling. | done | `scripts/manual/key-help.sh` (screenshot: the panel is about 740×290 logical px) |

## Default bindings

<!-- spec:keybindings:begin -->
| Name | Action | Default keys |
|------|--------|--------------|
| `new-window` | `win.new-window` | `<Ctrl>n` |
| `close-window` | `window.close` | `<Ctrl>w`, `q` |
| `quit` | `app.quit` | `<Ctrl>q` |
| `open` | `win.open` | `Return`, `KP_Enter`, `space` |
| `rename` | `win.rename` | `F2` |
| `view-icons` | `win.view::icons` | `<Ctrl>1` |
| `view-details` | `win.view::details` | `<Ctrl>2` |
| `toggle-hidden` | `win.toggle-hidden` | `<Ctrl>h` |
| `go-up` | `win.go-up` | `<Alt>Up`, `BackSpace` |
| `go-back` | `win.go-back` | `<Alt>Left` |
| `go-forward` | `win.go-forward` | `<Alt>Right` |
| `go-home` | `win.go-home` | `<Alt>Home` |
| `reload` | `win.reload` | `F5`, `<Ctrl>r` |
| `select-all` | `win.select-all` | `<Ctrl>a` |
| `unselect-all` | `win.unselect-all` | `Escape` |
| `copy` | `win.copy` | `<Ctrl>c` |
| `cut` | `win.cut` | `<Ctrl>x` |
| `paste` | `win.paste` | `<Ctrl>v` |
| `trash` | `win.trash` | `Delete` |
| `delete` | `win.delete` | `<Shift>Delete` |
| `new-folder` | `win.new-folder` | `<Ctrl><Shift>n` |
| `open-with` | `win.open-with` | |
| `properties` | `win.properties` | `<Ctrl>i`, `<Alt>Return` |
| `context-menu` | `win.context-menu` | `Menu`, `<Shift>F10` |
| `zoom-in` | `win.zoom-in` | `<Ctrl>plus`, `<Ctrl>equal`, `<Ctrl>KP_Add` |
| `zoom-out` | `win.zoom-out` | `<Ctrl>minus`, `<Ctrl>KP_Subtract` |
| `zoom-reset` | `win.zoom-reset` | `<Ctrl>0`, `<Ctrl>KP_0` |
| `location` | `win.location` | `<Ctrl>l` |
| `toggle-sidebar` | `win.toggle-sidebar` | `F9` |
| `add-bookmark` | `win.add-bookmark` | `<Ctrl>d` |
| `sort-by-name` | `win.sort-by::name` | |
| `sort-by-size` | `win.sort-by::size` | |
| `sort-by-modified` | `win.sort-by::modified` | |
| `sort-by-type` | `win.sort-by::type` | |
| `sort-reversed` | `win.sort-reversed` | |
| `show-help` | `win.show-help` | `question` |
<!-- spec:keybindings:end -->

## Implementation notes

- The table lives in code as `keys::BINDINGS`, in the same order, with the overlay description (`KEY-8`) as an extra field. To add a binding, edit this table, `keys::BINDINGS`, `data/keybindings.example.ini` and the man page `data/junk.1` (`INT-32`). The spec tests fail until all four agree.
- `KEY-3`: unmodified bindings are capture-phase shortcuts on the main window (`keys::attach_local`). Each one is a `CallbackAction` that returns `Proceed` when the focused widget is a `gtk::Editable`. They live on the window, not the view, because changing folder leaves focus nowhere (`NAV-30`). The rename dialog is a separate window, so it never sees them.
- `KEY-9`: `help::attach` adds a capture key controller and a capture click gesture to the window after `keys::attach_local`. The window runs its newest controllers first, so they see events before the app accelerators and the local bindings. Modifier keys are matched by keysym, because `KeyEvent::is_modifier` is false for `wtype`'s virtual keyboard.
