# Roadmap

The order of `planned` requirements. Change it here when priorities change.

Nothing is planned.

## Verification backlog

Built requirements that are not fully verified. Clear these before starting new planned work.

- `VIEW-13`, `NAV-40`…`NAV-42`: re-run `scripts/manual/sort.sh` and `scripts/manual/sidebar.sh` on Hyprland after the remembered-preferences work. Their keyboard checks pass headless; their pointer clicks don't register there yet.
- `OPS-13`: partial on Hyprland 0.56 with the GL renderer (the compositor drops the drag-icon hotspot). Re-check after a Hyprland update.
- `OPS-1`: planned. A press-and-immediate-drag on an item must start a drag, not a rubberband. Check with `scripts/manual/drag-hit.sh` on Hyprland.
