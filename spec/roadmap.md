# Roadmap

The order of approved work: `planned` requirements, and `partial` ones with a known bug from `/bug`. Change it here when priorities change. `/implement` works through this file: the verification backlog first, then the requirements in the order listed.

1. `LOOK-21`: error messages as `Could not <verb> “<name>”` (the Open With failure has no quotes).
2. `LOOK-22`: scrollbars of the file views and other scrolled areas styled like the sidebar's, per the style guide.
3. `NAV-31`: jump to the first item starting with a typed letter when sorted by name; `close-window` loses its `q` default (`KEY` table; also `KEY-9`'s check in `key-help.sh` uses `q`).

Keep the guide's copies of values in sync when this app's look changes (see `PROD-12`).

## Verification backlog

Built requirements that are not fully verified. Clear these before starting new planned work.

- `VIEW-13`, `NAV-40`…`NAV-42`: re-run `scripts/manual/sort.sh` and `scripts/manual/sidebar.sh` on Hyprland after the remembered-preferences work. Their keyboard checks pass headless; their pointer clicks don't register there yet.
- `OPS-13`: partial on Hyprland 0.56 with the GL renderer (the compositor drops the drag-icon hotspot). Re-check after a Hyprland update.
