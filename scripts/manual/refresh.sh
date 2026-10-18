#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim wlrctl
# Spec: NAV-13. Run from the repo root after `nix build`, with no Junk Drawer windows
# open, preferably through scripts/manual/headless.sh. Changes a folder behind an open
# window, including a file renamed twice in a row and a file that is gone at once, then
# checks that the window already looks as it does after `reload`. Screenshots go to $OUT.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"; trap 'cleanup; rm -rf "$T/demo" "$T/cfg"' EXIT
D=$T/demo; mkdir -p "$D"; touch "$D/alpha.txt" "$D/bravo.txt" "$D/charlie.txt"
mkdir -p "$T/cfg"; export XDG_CONFIG_HOME=$T/cfg

launch "$D"
sleep 0.5; shot "$OUT/1-start.png"

mv "$D/alpha.txt" "$D/alpha2.txt"; mv "$D/alpha2.txt" "$D/alpha3.txt"
touch "$D/temp.txt"; rm "$D/temp.txt"
mkdir "$T/elsewhere"; mv "$D/bravo.txt" "$T/elsewhere/"; mv "$T/elsewhere/bravo.txt" "$D/delta.txt"
sleep 1; shot "$OUT/2-after-changes.png"

touch "$D/echo.txt"; mv "$D/charlie.txt" "$D/foxtrot.txt"
sleep 1; shot "$OUT/3-after-later-changes.png"

keys -k F5; sleep 1; shot "$OUT/4-after-reload.png"
check "the window matches a reload after the changes" "same" \
  "$(cmp -s "$OUT/3-after-later-changes.png" "$OUT/4-after-reload.png" && echo same || echo different)"
check "the changes show: not like the start" "different" \
  "$(cmp -s "$OUT/1-start.png" "$OUT/3-after-later-changes.png" && echo same || echo different)"
exit $FAILS
