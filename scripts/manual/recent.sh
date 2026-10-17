#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim dbus wl-clipboard
# Spec: INT-12, NAV-40, OPS-35. Run from the repo root after `nix build`, with no Junk Drawer
# windows open. Junk Drawer runs on a private session bus (dbus-run-session) with a temp
# XDG_DATA_HOME, so gvfs serves a scratch recently-used.xbel and the real Recent list
# is never touched. Ctrl+C/wl-paste read the selection, which replaces the clipboard.
# Screenshots go to $OUT.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
trap 'for p in $(wins | jq -r ".[].pid" | sort -u); do kill "$p"; done; kill "${BUS:-}" 2>/dev/null; rm -rf "$T/demo" "$T/data"' EXIT
D=$T/demo; mkdir -p "$D" "$T/data"; echo hi > "$D/a.txt"; echo bye > "$D/b.txt"
X=$T/data/recently-used.xbel
entry() { # entry URI MIME
  cat <<XML
  <bookmark href="$1" added="2026-10-17T00:00:00Z" modified="2026-10-17T00:00:00Z" visited="2026-10-17T00:00:00Z">
    <info><metadata owner="http://freedesktop.org">
      <mime:mime-type type="$2"/>
      <bookmark:applications><bookmark:application name="test" exec="&apos;test %u&apos;" modified="2026-10-17T00:00:00Z" count="1"/></bookmark:applications>
    </metadata></info>
  </bookmark>
XML
}
{ echo '<?xml version="1.0" encoding="UTF-8"?>'
  echo '<xbel version="1.0" xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks" xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info">'
  entry "file://$D/a.txt" text/plain; entry "file://$D/b.txt" text/plain
  echo '</xbel>'; } > "$X"
title() { active .title; }
in_list() { grep -q "$1\"" "$X" && echo listed || echo gone; }

XDG_DATA_HOME=$T/data dbus-run-session -- "$BIN" "$D" >/dev/null 2>&1 & BUS=$!
for _ in $(seq 40); do [ "$(win_count)" -gt 0 ] && break; sleep 0.2; done; sleep 0.5
hyprctl dispatch "hl.dsp.focus({ window = \"class:$APP\" })" >/dev/null; sleep 0.3
shot "$OUT/1-sidebar.png"
keys -M ctrl l -m ctrl; keys "recent:///" -k Return; sleep 1
check "Recent opens"                     "recent:///" "$(title)"
shot "$OUT/2-recent.png"
# gvfs lists only files in recent:///, not folders.
keys -k Home -M ctrl c -m ctrl; sleep 0.2
check "copy gives the real file"         "file://$D/a.txt" "$(wl-paste -n -t text/uri-list | tr -d '\r')"
keys -k Menu; shot "$OUT/3-menu.png"; keys -k Escape
keys -k Delete; sleep 1
check "Delete removes the entry"         gone "$(in_list "file://$D/a.txt")"
check "and keeps the file"               hi "$(cat "$D/a.txt")"
keys -k Home -M shift -k Delete -m shift; sleep 1
check "Shift+Delete removes without asking" "gone recent:///" "$(in_list "file://$D/b.txt") $(title)"
check "and keeps the file"               bye "$(cat "$D/b.txt")"

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
