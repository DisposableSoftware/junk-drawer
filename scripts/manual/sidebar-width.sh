#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim wlrctl imagemagick
# Spec: LOOK-12. Run from the repo root after `nix build`, with no Junk Drawer windows open.
# Measures the sidebar from screenshots: 16px between its edges and the widest row,
# a 280px cap with a long bookmark, no line at its edge, and a scrollbar that never
# covers a row. Opens the test window on
# workspace ${WS:-7} and goes back afterwards. Screenshots go to $OUT.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
PREV=$(hyprctl activeworkspace -j | jq -r .name)
trap 'cleanup; hyprctl dispatch "hl.dsp.focus({ workspace = \"$PREV\" })" >/dev/null; rm -rf "$T/demo" "$T/cfg"' EXIT
D=$T/demo; mkdir -p "$D/Alpha" "$T/cfg/gtk-3.0" "$T/cfg/junk-drawer"
printf '[appearance]\nopacity = 1.0\n' > "$T/cfg/junk-drawer/config.ini"
printf 'file://%s Bm\n' "$D/Alpha" > "$T/cfg/gtk-3.0/bookmarks"
printf 'XDG_DESKTOP_DIR="$HOME/"\nXDG_DOCUMENTS_DIR="%s"\n' "$D/Alpha" > "$T/cfg/user-dirs.dirs"
hyprctl dispatch "hl.dsp.focus({ workspace = \"${WS:-7}\" })" >/dev/null; sleep 0.5

measure() { shot "$OUT/$1.png"; sidebar_measure "$OUT/$1.png"; }
near() { [ "$(( $1 - $2 < 0 ? $2 - $1 : $1 - $2 ))" -le "${3:-2}" ] && echo yes || echo "no ($1)"; }

launch "$D" "$T/cfg"
wlrctl pointer move 2000 0; sleep 0.3
read -r W L R LINE < <(measure 1-short)
check "left padding is 16px"            yes "$(near "$L" 16)"
check "right padding is 16px"           yes "$(near "$R" 16)"
check "no line at the sidebar's edge"   0 "$LINE"
printf 'file://%s A bookmark with a really very extremely long name that goes on and on\n' "$D/Alpha" >> "$T/cfg/gtk-3.0/bookmarks"; sleep 1
read -r W2 _ _ LINE2 < <(measure 2-long)
check "a long name caps it at 280px"    yes "$(near "$W2" 280)"
check "no line with a long name"       0 "$LINE2"

# The scrollbar never covers a row (LOOK-12): with 40 bookmarks the rows overflow.
# File System (the current row, near the top) lies beside the scrollbar's slider;
# hover the scrollbar so it is at its widest, then on the row's middle line the
# slider must start right of where the row's highlight ends.
cleanup; PID=; sleep 0.5
for i in $(seq -w 1 40); do mkdir -p "$D/F$i"; printf 'file://%s Folder %s\n' "$D/F$i" "$i"; done >> "$T/cfg/gtk-3.0/bookmarks"
launch / "$T/cfg"
W3=$W2   # the long bookmark is still there; rows now fill the height, so measure can't see the edge
FS_Y=156   # File System: Recent, Home, Docs (Alpha), then it
read -r x y < <(wins | jq -r '.[0] | "\(.at[0]) \(.at[1])"')
read -r cx cy < <(hyprctl cursorpos -j | jq -r '"\(.x) \(.y)"')
wlrctl pointer move $((x + W3 - 3 - cx)) $((y + FS_Y - cy)); wlrctl pointer move 0 2; sleep 0.5
shot "$OUT/3-scroll.png"
GAP=$(magick "$OUT/3-scroll.png" -crop "$((W3 * 2))x1+0+$(( (FS_Y + 2) * 2 ))" -depth 8 txt:- | awk -F'[(,)]' -v w="$W3" '
  NR==1 { next }
  { split($0, a, ":"); px = a[1] + 0; r = $2; g = $3; b = $4; l = (r + g + b) / 3; L[px] = l; R[px] = r; G[px] = g; B[px] = b }
  END {
    hl = 20                                  # inside the row highlight, left of the icon
    for (i = 2 * w - 60; i < 2 * w; i++)     # the last 30 logical px
      if ((R[i]-R[hl])^2 + (G[i]-G[hl])^2 + (B[i]-B[hl])^2 < 60) last = i
    # Anything else inside the highlight there is the scrollbar on top of it.
    for (i = 2 * w - 60; i < last; i++)
      if ((R[i]-R[hl])^2 + (G[i]-G[hl])^2 + (B[i]-B[hl])^2 > 300) { print "covered at " int(i/2) "px"; exit }
    print "clear"
  }')
check "the scrollbar doesn't cover a row" clear "$GAP"
echo "short: ${W}px wide, ${L}px left, ${R}px right; long: ${W2}px; screenshots: $OUT"
exit $FAILS
