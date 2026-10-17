#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim wlrctl wl-clipboard
# Spec: VIEW-13. Run from the repo root after `nix build`, with no Junk Drawer windows open.
# Drives the folder menu's Sort By from the keyboard (Menu, then Down to Sort By,
# Right into the submenu) and reads the first item back through Ctrl+C and wl-paste,
# which replaces the clipboard. Screenshots go to $OUT.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"; trap 'cleanup; rm -rf "$T/demo"' EXIT
D=$T/demo; mkdir -p "$D"
head -c 300 /dev/zero > "$D/a"; head -c 100 /dev/zero > "$D/b"; head -c 200 /dev/zero > "$D/c"
touch -d '2024-01-02' "$D/a"; touch -d '2024-01-03' "$D/b"; touch -d '2024-01-01' "$D/c"
title() { active .title; }
first() { keys -k Escape -k Home -M ctrl c -m ctrl; sleep 0.2; basename "$(wl-paste -n -t text/uri-list | tr -d '\r')"; }
# sort_by N: the Nth entry of the Sort By submenu (1 Name, 2 Size, 3 Modified, 4 Type,
# 5 Reversed Order)
sort_by() {
  keys -k Escape -k Menu -k Down -k Down -k Down -k Down -k Right
  for _ in $(seq "$1"); do keys -k Down; done   # the submenu opens on its back header
  keys -k Return; sleep 0.3
}

launch "$D"
check "name order by default"     a "$(first)"
keys -k Escape -k Menu -k Down -k Down -k Down -k Down -k Right; shot "$OUT/1-submenu.png"; keys -k Escape -k Escape
sort_by 2;                        check "Size: smallest first"      b "$(first)"
sort_by 5;                        check "Reversed: largest first"   a "$(first)"
sort_by 3;                        check "Modified keeps Reversed"   b "$(first)"
sort_by 5;                        check "Modified, oldest first"    c "$(first)"

# A details-view header click shows up in the menu.
keys -M ctrl 2 -m ctrl
read -r X Y < <(wins | jq -r '.[0] | "\(.at[0]) \(.at[1])"')
read -r CX CY < <(hyprctl cursorpos -j | jq -r '"\(.x) \(.y)"')
wlrctl pointer move $((X + 260 + 5 - CX)) $((Y + 52 + 5 - CY)); wlrctl pointer move -5 -5; sleep 0.3
wlrctl pointer click left; sleep 0.4
check "the Name header sorts by name" a "$(first)"
keys -k Escape -k Menu -k Down -k Down -k Down -k Down -k Right; shot "$OUT/2-after-header.png"

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
