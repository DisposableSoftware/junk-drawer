#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim imagemagick zip
# Spec: INT-7, VIEW-37. Run from the repo root after `nix build`, with no Junk Drawer
# windows open; headless by default (scripts/manual/headless.sh), which runs at scale 2.
# Uses a temp XDG_CACHE_HOME, XDG_DATA_HOME and XDG_STATE_HOME. A slow fake thumbnailer
# writes its PNG at once and exits later, so two sizes of one file overlap the way a
# real thumbnailer did on a HiDPI first show.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
trap 'cleanup; rm -rf "$T"' EXIT
export XDG_CACHE_HOME=$T/cache XDG_STATE_HOME=$T/state XDG_DATA_HOME=$T/data
D=$T/demo; mkdir -p "$D" "$T/data/thumbnailers"
magick -size 300x450 xc:'#89b4fa' "$T/cover.png"
(cd "$T" && zip -q "$D/book.cbz" cover.png)

# The user data dir comes first, so this wins over any system thumbnailer for comics.
cat > "$T/slow.sh" <<EOF
#!/usr/bin/env bash
echo "start \$1 \$(date +%s.%N)" >> "$T/runs"
cp "$T/cover.png" "\$3"
sleep \$([ "\$1" -le 128 ] && echo 2 || echo 3)
echo "end \$1 \$(date +%s.%N)" >> "$T/runs"
EOF
chmod +x "$T/slow.sh"
printf '[Thumbnailer Entry]\nTryExec=%s\nExec=%s %%s %%i %%o\nMimeType=application/vnd.comicbook+zip;application/x-cbz;\n' \
  "$T/slow.sh" "$T/slow.sh" > "$T/data/thumbnailers/slow.thumbnailer"

uri="file://$D/book.cbz"   # temp paths need no escaping
md5=$(printf %s "$uri" | md5sum | cut -d' ' -f1)
normal=$XDG_CACHE_HOME/thumbnails/normal/$md5.png
large=$XDG_CACHE_HOME/thumbnails/large/$md5.png
fail=$XDG_CACHE_HOME/thumbnails/fail/junk-drawer/$md5.png
exists() { [ -e "$1" ] && echo yes || echo no; }

# Both sizes at once: 64px icons at scale 2 ask for normal; zooming to 96px asks for
#    large while normal is still running.
mkdir -p "$T/cfg/junk-drawer"; printf '[view]\nicon-size=64\n' > "$T/cfg/junk-drawer/config.ini"
launch "$D" "$T/cfg"
sleep 0.5
keys -M ctrl -k plus -m ctrl
sleep 5
check "the two sizes ran at the same time" 2 "$(grep -c '^start' "$T/runs")"
check "the normal thumbnail is cached" yes "$(exists "$normal")"
check "the large thumbnail is cached" yes "$(exists "$large")"
check "no failure is recorded" no "$(exists "$fail")"
shot "$OUT/1-both-sizes.png"

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
