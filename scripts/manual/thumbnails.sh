#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim imagemagick
# Spec: VIEW-37, INT-7, CONF-16. Run from the repo root after `nix build`, with no Junk Drawer
# windows open. Uses a temp XDG_CACHE_HOME, so the real thumbnail cache is untouched,
# and checks what lands in it. Screenshots go to $OUT.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
trap 'cleanup; rm -rf "$T/demo" "$T/cache" "$T/cfg"' EXIT
export XDG_CACHE_HOME=$T/cache
D=$T/demo; mkdir -p "$D"
magick -size 800x400 gradient:'#f38ba8-#89b4fa' "$D/wide.png"
magick -size 300x600 gradient:'#a6e3a1-#f9e2af' "$D/tall.jpg"
magick -size 400x400 xc:'#a6e3a1' "$D/stale.png"
magick -size 600x800 xc:white -fill '#1e1e2e' -pointsize 60 -draw "text 80,200 'PDF page'" "$D/doc.pdf"
echo text > "$D/notes.txt"
mkdir -p "$T/cfg/junk-drawer"; printf '[view]\nshow-thumbnails=false\n' > "$T/cfg/junk-drawer/config.ini"

uri() { echo "file://$1"; }   # temp paths need no escaping
md5() { printf %s "$1" | md5sum | cut -d' ' -f1; }
thumb() { echo "$XDG_CACHE_HOME/thumbnails/large/$(md5 "$(uri "$1")").png"; }
mtime_of() { magick identify -format '%[Thumb::MTime]' "$1" 2>/dev/null; }
# A stale thumbnail (wrong mtime, blue) that must be replaced.
mkdir -p "$XDG_CACHE_HOME/thumbnails/large"
magick -size 256x256 xc:'#89b4fa' -set 'Thumb::MTime' 1 -set 'Thumb::URI' "$(uri "$D/stale.png")" "$(thumb "$D/stale.png")"

launch "$D"
sleep 3; shot "$OUT/1-grid.png"
for f in wide.png tall.jpg doc.pdf; do
  check "$f thumbnail is cached with its mtime" "$(stat -c %Y "$D/$f")" "$(mtime_of "$(thumb "$D/$f")")"
done
check "a stale thumbnail is replaced" "$(stat -c %Y "$D/stale.png")" "$(mtime_of "$(thumb "$D/stale.png")")"
check "plain text gets none, and no fail entry" "no no" \
  "$([ -e "$(thumb "$D/notes.txt")" ] && echo yes || echo no) $([ -e "$XDG_CACHE_HOME/thumbnails/fail/junk-drawer/$(md5 "$(uri "$D/notes.txt")").png" ] && echo yes || echo no)"
check "the cache is private (0600)" 600 "$(stat -c %a "$(thumb "$D/wide.png")")"
keys -M ctrl 2 -m ctrl; sleep 1; shot "$OUT/2-details.png"
cleanup; sleep 0.5

launch "$D" "$T/cfg"
sleep 2; shot "$OUT/3-thumbnails-off.png"

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
