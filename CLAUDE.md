# Junk Drawer

A lightweight, minimal Linux file manager built for Hyprland. Display name **Junk Drawer**, binary `junk`, app ID `io.github.DisposableSoftware.JunkDrawer`.

**What the app does is defined in [`spec/`](spec/README.md).** This file covers how to work on it.

## Spec-first workflow (mandatory)

Every change follows these steps, in order. For feature requests, use the `/feature` skill (`.claude/skills/feature/`), which adds a user-approval step between the spec and the code. For bugs, use `/bug` (`.claude/skills/bug/`): it reproduces the bug, finds the spec gap that let it happen, then commits the spec delta and the fix without an approval step, unless the fix changes intended behaviour.

1. **Spec.** Before touching code, update the relevant `spec/*.md`.
   - **New behaviour or a changed requirement:** add or edit a requirement row. A new row gets the next free ID in its prefix, with status `planned`. If the request is unclear, settle the wording with the user before coding.
   - **Bug fix:** find the requirement the bug breaks. If no requirement covers the behaviour, the bug is a gap in the spec, so add one (e.g. `NAV-30` came from the Backspace-after-entering-a-folder bug).
   - **Refactor with no behaviour change:** no spec edit, but it must still pass step 3.
   - Commit the spec change on its own, before the code: `spec: <summary>`.
2. **Code.** Implement only what the spec says. If the code needs behaviour the spec doesn't describe, go back to step 1.
3. **Confirm against the spec.** Before the code commit:
   - Run `cargo test`. It includes `src/spec_tests.rs`, which checks the keybinding and config tables in the spec against the code and `data/config.example.ini`.
   - For every requirement ID touched, verify it as its **Verified by** column says (unit test, or a manual/screenshot check on Hyprland). Prefer adding a unit test where one is possible.
   - Re-read the changed code against the requirement text and look for behaviour the spec doesn't mention.
   - Update the row: set the status to `done` (or `partial` with the gap written in), and fill in **Verified by**. Update `roadmap.md` if a planned item landed.
4. **Commit** the code with a `Spec: ID, ID` trailer listing the requirements it implements or affects, and tell the user which IDs were verified and how.

When reporting to the user, say plainly if something could only be partly verified (e.g. the user had Junk Drawer open, so the manual test was skipped).

## Commands

```bash
nix develop                    # devshell: cargo, clippy, rustfmt, rust-analyzer, gtk4
cargo run                      # run (inside devshell)
cargo clippy --all-targets -- -D warnings    # must pass
cargo fmt
cargo test                     # unit tests + spec conformance tests
nix build                      # package -> ./result/bin/junk (also runs the tests)
nix flake check
git config core.hooksPath .githooks   # once per clone: enables the pre-push release check
```

The flake only sees git-tracked files. Run `git add` on new files before `nix build` or `nix develop`. After adding crates, commit `Cargo.lock`, which `nix/package.nix` reads through `cargoLock.lockFile`. `spec/` is part of the package source, because the spec tests `include_str!` it.

## Code layout

```
src/main.rs        GtkApplication, app actions, prgname
src/cli.rs         --help and --version, before GTK starts                    INT-30, INT-31
src/config.rs      config.ini + keybindings.ini, first-run files, OnceLock    CONF KEY-2
src/state.rs       remembered view, sort, hidden, sidebar, zoom (state.ini)  CONF-18…20
src/theme.rs       Catppuccin palette or GTK theme colours, CSS provider     LOOK
src/window.rs      window, State (model + history), win.* actions, views     VIEW NAV
src/keys.rs        keys::BINDINGS + global/local attachment                  KEY
src/help.rs        keybinding help overlay (?)                               KEY-8…10
src/sort.rs        sorters, natural_cmp, sort-by actions                     VIEW-1x
src/zoom.rs        zoom levels, Ctrl+scroll                                  VIEW-25
src/caption.rs     grid captions, async folder item counts                   VIEW-33…35
src/thumbnail.rs   freedesktop thumbnails: cache, .thumbnailer, GdkPixbuf   VIEW-37, INT-7
src/dnd.rs         drag source, drop target, action choice                   OPS-1…5
src/ops.rs         async copy/move jobs (conflicts, progress), delete, trash OPS-6…12, OPS-30
src/conflict.rs    name-clash dialog: Replace/Merge, Skip, Rename, Cancel    OPS-11
src/progress.rs    transfer progress window, shown after 1 s, Cancel         OPS-12
src/clipboard.rs   cut/copy/paste                                            OPS-2x
src/rename.rs      rename dialog                                             OPS-32, OPS-33
src/dialog.rs      shared small windows: name prompt, Escape to close, errors
src/menu.rs        context menus (right-click, Menu key)                     OPS-35
src/location.rs    location entry (Ctrl+L): resolving, Tab completion        NAV-16
src/fm1.rs         org.freedesktop.FileManager1 D-Bus service                NAV-5, INT-11
src/sidebar.rs     sidebar: places, bookmarks, volumes (mount/eject), F9     NAV-40…43, INT-8…10
src/openwith.rs    Open With dialog                                          OPS-36
src/properties.rs  Properties window, background size count                  OPS-36
src/spec_tests.rs  spec ↔ code conformance tests
scripts/manual/    UI checks (lib.sh helpers; headless.sh runs one in a headless sway)
scripts/install.sh install without Nix (PREFIX, DESTDIR, --uninstall)      INT-40
scripts/release-check.sh  recommended version bump since the last v* tag (spec delta)
scripts/release.sh  bump, commit "Junk Drawer X.Y.Z", tag, draft notes (no push)
.githooks/pre-push  blocks history-backup; on a push of main offers to release
scripts/sync-desktop.sh  dev only: desktop entry + icons for ./result into ~/.local
data/              desktop entry, metainfo, icons, style.css, themes/<flavor>.css,
                   config.example.ini + keybindings.example.ini (first-run files) CONF-17
                   junk.1 (man page, checked by spec_tests)               INT-32
nix/               package.nix, nixos-module.nix, hm-module.nix              INT-2x
spec/              the specification
```

## Conventions

- Keep the dependency list short. Ask before adding a crate, and prefer GIO/GLib equivalents (`PROD-1`).
- Use async GIO APIs for file I/O. Never block the main loop.
- Avoid strong reference cycles in signal closures: use `glib::clone!` with `#[weak]` for widgets and models.
- Look up gtk4-rs and GTK docs before guessing at APIs, because the bindings change between minor versions. Read the vendored source in `~/.cargo/registry/src/*/gtk4-0.11.*` for exact signatures.
  - https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/
  - https://docs.gtk.org/gtk4/
- To match Nautilus, read its source (`nix build nixpkgs#nautilus.src`) rather than guessing.
- Comments only when the reason is not obvious.
- Unit tests that use GTK must call `gtk::init()` and skip if it fails, because the `nix build` sandbox has no display. GTK may only be initialised on one thread, so keep GTK checks in one test.

## Manual testing

Prefer adding a script to `scripts/manual/` (source `lib.sh`, which has the guards below built in) over ad-hoc commands. Each script names the spec IDs it checks. Run it after `nix build`.

### Headless (default)

Run checks with `./scripts/manual/headless.sh ./scripts/manual/<name>.sh`. It starts a headless sway with its own D-Bus session and runtime dir, so it works while the screen is locked or the user has Junk Drawer open, and never touches their windows. `lib.sh` switches to `swaymsg` when `JD_HEADLESS` is set, so scripts don't change.

- The output defaults to 3840×2160 at scale 2, like bayle's monitor. Set `JD_RES` and `JD_SCALE` to test other scales; at scale 1 thin strokes in small text render faint, which is not a layout bug.
- Send screenshots (`shot`, into `$OUT`) to the user when a change is visual.
- Headless sway drops the first key after `wtype` loads its keymap; `keys` sends a Shift tap first to absorb it.
- Headless sway is not Hyprland. Use a real Hyprland session (below) for compositor behaviour: window states, focus rules, drag icons (`OPS-13`).

### On Hyprland

Run `./scripts/manual/<name>.sh` directly.

- Screenshots: `hyprctl clients -j` gives the window geometry, and `grim -g` captures it. Use an `XDG_CONFIG_HOME` in a temp dir to test config variants.
- Check `pidof hyprlock` first: if the screen is locked, grim captures the lock screen. `systemd-inhibit --what=idle` works, because hypridle respects inhibitors.
- New windows don't take focus. Run `hyprctl dispatch 'hl.dsp.focus({ window = "class:io.github.DisposableSoftware.JunkDrawer" })'` (Hyprland 0.56 uses a Lua config), then send keys with `wtype`. Always check `hyprctl activewindow` first, so keys never land in the user's other apps.
- The Nix wrapper renames the process to `.junk-wrapped`, so `pgrep -x junk` never matches, and `pgrep -f` matches the test's own shell. Find and kill test windows through `hyprctl clients -j` (`.pid`).
- The app is single-instance. If the user already has Junk Drawer open, a test launch only opens a window in *their* process, and killing that pid closes their windows. Check there are no `io.github.DisposableSoftware.JunkDrawer` clients before testing, and don't go ahead if there are.
