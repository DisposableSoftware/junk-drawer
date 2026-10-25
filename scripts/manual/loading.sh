#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim
# Spec: VIEW-38, LOOK-23. Run from the repo root after `nix build`, headless:
# ./scripts/manual/headless.sh ./scripts/manual/loading.sh
# A folder of many files loads slowly enough to show the throbber; screenshots go to $OUT.
# Look at them: spinner at the right end of the path bar while loading, none in a small folder or afterwards.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
trap 'cleanup; rm -rf "$T/small" "$T/big"' EXIT
mkdir -p "$T/small/a" "$T/big"
(cd "$T/big" && seq 1 ${JD_FILES:-200000} | xargs touch)

launch "$T/small"
shot "$OUT/1-small.png"
cleanup; sleep 0.5

launch "$T/big"
for i in 1 2 3; do shot "$OUT/2-big-$i.png"; done
sleep 20
shot "$OUT/3-big-loaded.png"
keys -k Tab -k BackSpace;               check "focus stays on the view"        "$T" "$(active .title)"

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
