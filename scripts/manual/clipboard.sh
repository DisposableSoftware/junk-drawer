#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim wl-clipboard
# Spec: OPS-20, OPS-21, OPS-22. Run from the repo root after `nix build`, with no Junk Drawer
# windows open. Uses wl-clipboard as the "other app", so this overwrites the clipboard.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); trap 'cleanup; rm -rf "$T"' EXIT
D=$T/here; S=$T/elsewhere; mkdir -p "$D" "$S"
echo a > "$D/a.txt"; echo b > "$D/b.txt"; echo c > "$S/c.txt"; echo u > "$S/u.txt"
uri() { printf 'file://%s' "$1"; }
title() { active .title; }

launch "$D"
keys -M ctrl a -m ctrl -M ctrl c -m ctrl
check "copy: gnome-copied-files" "$(printf 'copy\n%s\n%s' "$(uri "$D/a.txt")" "$(uri "$D/b.txt")")" \
  "$(wl-paste -n -t x-special/gnome-copied-files)"
check "copy: uri-list offered" "yes" "$(wl-paste --list-types | grep -qx text/uri-list && echo yes)"
keys -M ctrl x -m ctrl
check "cut: flag" "cut" "$(wl-paste -n -t x-special/gnome-copied-files | head -1)"

# Another app cuts c.txt; pasting in Junk Drawer moves it and clears the clipboard.
printf 'cut\n%s' "$(uri "$S/c.txt")" | wl-copy -t x-special/gnome-copied-files
sleep 0.3; keys -k Escape -M ctrl v -m ctrl; sleep 1
check "paste of a cut moves the file" "yes no" \
  "$([ -e "$D/c.txt" ] && echo yes || echo no) $([ -e "$S/c.txt" ] && echo yes || echo no)"
check "clipboard cleared after the cut is pasted" "" \
  "$(wl-paste --list-types 2>/dev/null | grep -x x-special/gnome-copied-files)"

# A plain file list (no gnome flag) is copied.
uri "$S/u.txt" | wl-copy -t text/uri-list
sleep 0.3; keys -M ctrl v -m ctrl; sleep 1
check "uri-list paste copies" "yes yes" \
  "$([ -e "$D/u.txt" ] && echo yes || echo no) $([ -e "$S/u.txt" ] && echo yes || echo no)"

echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
