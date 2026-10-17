# Junk Drawer: specification

This folder is the source of truth for what Junk Drawer does. The code implements the spec. If the code and the spec disagree, the code is wrong, or the spec must be changed first.

## Files

| File | Prefix | Covers |
|------|--------|--------|
| [product.md](product.md) | `PROD` | Goals, non-goals, platform and stack constraints |
| [views.md](views.md) | `VIEW` | Icon and details views, layout, labels, sorting |
| [navigation.md](navigation.md) | `NAV` | Windows, opening items, history, hidden files, focus |
| [keybindings.md](keybindings.md) | `KEY` | Default bindings, scope rules, user overrides |
| [file-operations.md](file-operations.md) | `OPS` | Drag and drop, copy/move, clipboard, trash, rename |
| [appearance.md](appearance.md) | `LOOK` | Catppuccin theming, transparency, CSS layering |
| [configuration.md](configuration.md) | `CONF` | `config.ini` and `style.css` |
| [integration.md](integration.md) | `INT` | XDG/freedesktop specs, desktop entry, NixOS |
| [roadmap.md](roadmap.md) | — | The order of planned work |

## Requirement format

Each requirement is one line in a table:

| ID | Requirement | Status |
|----|-------------|--------|
| `VIEW-1` | One sentence that can be checked. | done |

- **IDs are permanent.** Do not renumber them or reuse one. When a requirement is dropped, keep its row, set the status to `dropped`, and give the reason.
- **Status** is one of:
  - `planned`: agreed but not built.
  - `partial`: built, but with a known gap. The gap is written in the row.
  - `done`: built and verified.
  - `dropped`: no longer wanted.
- Write behaviour, not implementation. Keep the "how" short, and put it under **Implementation notes** when it is not obvious or a past attempt failed.
- **Verified by** names the test, or says `manual` with what to check.

## Machine-checked sections

Tables between `<!-- spec:NAME:begin -->` and `<!-- spec:NAME:end -->` markers are parsed by `src/spec_tests.rs`. `cargo test` (and so `nix build`) fails when they disagree with the code or with `data/config.example.ini`. Keep their column layout unchanged.
