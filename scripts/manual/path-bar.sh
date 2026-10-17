#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim
# Spec: VIEW-30, VIEW-31, VIEW-32, VIEW-36, LOOK-7, CONF-13. Run from the repo root after
# `nix build`, with no Junk Drawer windows open. Screenshots go to $OUT (default: a temp dir);
# look at them to check the path text, its colour and padding, and the ellipsis.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
trap 'cleanup; rm -rf "$T/demo" "$T/long" "$T/cfg"' EXIT
D=$T/demo; mkdir -p "$D/Alpha"
LONG=$T/long; for i in 1 2 3 4 5 6 7 8; do LONG=$LONG/a-rather-long-folder-name-number-$i; done
mkdir -p "$LONG/the-current-folder"
mkdir -p "$T/cfg/junk-drawer"; printf '[view]\nshow-path=false\n' > "$T/cfg/junk-drawer/config.ini"
title() { active .title; }

launch "$D"
shot "$OUT/1-root.png"
keys -k Down -k Return;                 check "Return opens Alpha"              "$D/Alpha" "$(title)"
shot "$OUT/2-alpha.png"
keys -k Tab -k Tab -k Tab
shot "$OUT/3-after-tab.png"
keys -k BackSpace;                      check "keys still reach the view"       "$D" "$(title)"
cleanup; sleep 0.5

launch "$LONG/the-current-folder"
shot "$OUT/4-long.png"
cleanup; sleep 0.5

launch "$D" "$T/cfg"
shot "$OUT/5-show-path-false.png"
cleanup; sleep 0.5

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
