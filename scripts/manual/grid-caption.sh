#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim
# Spec: VIEW-33, VIEW-34, VIEW-35, LOOK-9, CONF-14. Run from the repo root after
# `nix build`, with no Junk Drawer windows open. Screenshots go to $OUT (default: a temp dir).
# Expected in 1-size.png: Empty "0 items", Locked "? items", Three "3 items",
# big.bin "3.4 MB", small.txt "512 bytes"; in 2-size-hidden.png: Three "4 items";
# in 3-reloaded.png (a file was added, then F5): Three "5 items".
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
trap 'cleanup; chmod 755 "$T/demo/Locked" 2>/dev/null; rm -rf "$T/demo" "$T/cfg"' EXIT
D=$T/demo; mkdir -p "$D/Empty" "$D/Locked" "$D/Three"
touch "$D/Three/a" "$D/Three/b" "$D/Three/c" "$D/Three/.hidden"; chmod 000 "$D/Locked"
head -c 512 /dev/zero > "$D/small.txt"; truncate -s 3400000 "$D/big.bin"
chmod 4755 "$D/big.bin"
title() { active .title; }

launch "$D"
shot "$OUT/1-size.png"
keys -M ctrl h -m ctrl;                 check "toggle hidden keeps the folder"  "$D" "$(title)"
sleep 0.5; shot "$OUT/2-size-hidden.png"
touch "$D/Three/d"; keys -k F5
sleep 0.5; shot "$OUT/3-reloaded.png"
cleanup; sleep 0.5

for v in modified type permissions none; do
  mkdir -p "$T/cfg/$v/junk-drawer"; printf '[view]\ngrid-caption=%s\n' "$v" > "$T/cfg/$v/junk-drawer/config.ini"
  launch "$D" "$T/cfg/$v"
  shot "$OUT/4-$v.png"
  cleanup; sleep 0.5
done

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
