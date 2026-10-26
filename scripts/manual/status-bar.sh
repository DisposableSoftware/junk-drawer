#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim
# Spec: VIEW-39, NAV-44, LOOK-24, CONF-23. Run from the repo root after `nix build`, headless:
# ./scripts/manual/headless.sh ./scripts/manual/status-bar.sh
# Screenshots go to $OUT; look at them: nothing at the bottom in 1-hidden, then "4 items, 3.0 kB"
# (three 1000-byte files and a folder) in 2-shown in the path bar's dimmed colour; 3-hidden-files
# adds the dotfile ("5 items, 4.0 kB"); 4-added shows a file written from outside; 5-config starts
# with the bar shown from config.ini (remember=false, since the F10 toggles above were remembered).
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
trap 'cleanup; rm -rf "$T/demo" "$T/cfg"' EXIT
D=$T/demo; mkdir -p "$D/Folder"
for f in a b c .hidden; do head -c 1000 /dev/zero > "$D/$f"; done
mkdir -p "$T/cfg/junk-drawer"; printf '[view]\nshow-status-bar=true\nremember=false\n' > "$T/cfg/junk-drawer/config.ini"
title() { active .title; }

launch "$D"
shot "$OUT/1-hidden.png"
keys -k F10; shot "$OUT/2-shown.png"
keys -k Down -k Return; check "the view keeps focus with the bar shown" "$D/Folder" "$(title)"
keys -k BackSpace
keys -M ctrl -k h -m ctrl; shot "$OUT/3-hidden-files.png"
head -c 5000 /dev/zero > "$D/added"; sleep 1; shot "$OUT/4-added.png"
keys -k F10; shot "$OUT/4b-hidden-again.png"
cleanup; sleep 0.5

launch "$D" "$T/cfg"
shot "$OUT/5-config.png"
cleanup; sleep 0.5

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
