#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim
# Spec: OPS-8, OPS-11, OPS-12, LOOK-14. Run from the repo root after `nix build`, with
# no Junk Drawer windows open. Pastes onto name clashes and answers the conflict dialog from
# the keyboard (focus starts on Replace; Shift+Tab walks back through Rename, Skip),
# then copies 40000 empty files to get a progress window and cancels it. Screenshots
# go to $OUT.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"; trap 'cleanup; rm -rf "$T/demo"' EXIT
D=$T/demo; mkdir -p "$D/src/d" "$D/out/d" "$D/many"
echo new > "$D/src/f"; echo inner > "$D/src/d/inside"; echo keep > "$D/out/d/keep"
(cd "$D/many" && seq 40000 | xargs touch)
reset() { echo old > "$D/out/f"; rm -f "$D/out/f (copy)"; }
title() { active .title; }
content() { cat "$D/out/$1" 2>/dev/null || echo missing; }

launch "$D/src"
keys -k Home -k Right -M ctrl c -m ctrl            # f
keys -k BackSpace -k Left -k Return;                check "in out" "$D/out" "$(title)"

reset; keys -M ctrl v -m ctrl; sleep 0.8;           check "a clash asks"        "Name Conflict" "$(title)"
shot "$OUT/1-replace.png"
keys -k Escape; sleep 0.5;                          check "Escape cancels"      old "$(content f)"
reset; keys -M ctrl v -m ctrl; sleep 0.8; keys -k Return; sleep 0.5
check "Enter replaces"  new "$(content f)"
reset; keys -M ctrl v -m ctrl; sleep 0.8; keys -M shift -k Tab -k Tab -m shift -k Return; sleep 0.5
check "Skip keeps the old file" old "$(content f)"
reset; keys -M ctrl v -m ctrl; sleep 0.8; keys -M shift -k Tab -m shift -k Return; sleep 0.5
check "Rename keeps both"       "old new" "$(content f) $(content 'f (copy)')"

keys -k BackSpace -k Right -k Return -k Home -M ctrl c -m ctrl   # src/d
keys -k BackSpace -k Left -k Return;                check "back in out" "$D/out" "$(title)"
keys -M ctrl v -m ctrl; sleep 0.8; shot "$OUT/2-merge.png"
keys -k Return; sleep 0.5
check "Merge adds the folder's items" "inner keep" "$(content d/inside) $(content d/keep)"

keys -k BackSpace -k Left -k Left -M ctrl c -m ctrl           # many
keys -k Right -k Return;                            check "in out again" "$D/out" "$(title)"
keys -M ctrl v -m ctrl
for _ in $(seq 30); do
  P=$(wins | jq -r '.[] | select(.title=="Copying") | .address' | head -1)
  [ -n "$P" ] && break; sleep 0.1
done
check "a long copy shows progress" yes "$([ -n "$P" ] && echo yes || echo no)"
if [ -n "$P" ]; then
  hyprctl dispatch "hl.dsp.focus({ window = \"address:$P\" })" >/dev/null; sleep 0.3
  shot "$OUT/3-progress.png"
  keys -k space; sleep 1
  n=$(ls "$D/out/many" 2>/dev/null | wc -l)
  check "Cancel stops it early" yes "$([ "$n" -lt 40000 ] && echo yes || echo no)"
  check "and closes the window" 0 "$(wins | jq '[.[] | select(.title=="Copying")] | length')"
fi

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
