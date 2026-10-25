#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim
# Spec: NAV-31. Run from the repo root after `nix build`, headless:
# ./scripts/manual/headless.sh ./scripts/manual/jump-letter.sh
# Return opens the selected item, so the title shows which item a letter selected and
# focused. Folders sort first, so the names are all folders. Screenshots go to $OUT.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"; trap 'cleanup; rm -rf "$T/demo" "$T/cfg"' EXIT
D=$T/demo; mkdir -p "$D"
for n in alpha Apple beta cherry delta; do mkdir -p "$D/$n"; done
for i in $(seq -w 1 60); do mkdir -p "$D/m$i"; done
mkdir -p "$D/zulu"
mkdir -p "$T/cfg/junk-drawer"
printf '[view]\ndefault=details\n' > "$T/cfg/junk-drawer/config.ini"
title() { active .title; }
# opens whatever is focused, then goes back up
pick() { keys -k Return; local t; t=$(title); keys -k BackSpace; echo "$t"; }

for cfg in "" "$T/cfg"; do
  name=${cfg:+details}; name=${name:-icons}
  launch "$D" $cfg
  keys c;                               check "$name: c selects cherry"           "$D/cherry" "$(pick)"
  keys z;                               check "$name: z scrolls to the last item" "$D/zulu" "$(pick)"
  shot "$OUT/$name-z.png"
  keys A;                               check "$name: A is case-blind, first is alpha" "$D/alpha" "$(pick)"
  keys q;                               check "$name: no match, nothing happens"  "$D" "$(title)"
  keys -k Tab -k Tab;                   # focus stays on the view
  keys b;                               check "$name: b selects beta"             "$D/beta" "$(pick)"
  keys -M ctrl c -m ctrl;               check "$name: Ctrl+letter does not jump"  "$D" "$(title)"
  cleanup; sleep 0.5
done

launch "$D"
keys d
check "d selects delta" "$D/delta" "$(pick)"
cleanup; sleep 0.5

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
