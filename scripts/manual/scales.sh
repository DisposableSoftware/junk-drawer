#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim imagemagick
# Spec: LOOK-19, LOOK-20. Run from the repo root after `nix build`. Runs headless at
# scales 1, 1.25, 1.5 and 2 (each in its own headless sway), takes screenshots of both
# views into $OUT, and compares layout sizes in logical px with scale 2, the finest grid.
# Positions are found from the first grid cell (the "Folder" folder) and the sidebar.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh

# f_top PNG REGION LINE: for the LINE-th text line (from 1) in REGION (device px), the F
# that starts it: right of the stem, the brightest row in its top three (the top bar)
# over the brightest row of the line (top or middle bar). A bar split across two pixel
# rows is about half as bright.
f_top() {
  magick "$1" -crop "$2" +repage -colorspace Gray -depth 8 txt:- | awk -F'[,: ()]+' -v s="$s" -v n="$3" '
    NR > 1 { v[$1, $2] = $3; if ($3 >= 60) ink[$2] = 1; w = $1 > w ? $1 : w; h = $2 > h ? $2 : h }
    END { y = 0
          for (i = 1; i <= n; i++) {
            for (; y <= h && !(y in ink); y++); y0 = y
            for (; y <= h && (y in ink); y++); y1 = y - 1 }
          for (x = 0; x <= w; x++) { for (y = y0; y <= y1; y++) if (v[x, y] >= 60) break; if (y <= y1) break }
          x0 = x
          for (y = y0; y <= y1; y++) { t = 0; c = 0
            for (x = x0 + int(2 * s); x <= x0 + int(4 * s); x++) { t += v[x, y]; c++ }
            m = t / c; if (m > best) best = m; if (y < y0 + 3 * s && m > top) top = m }
          printf "%.2f", (best > 0 ? top / best : 0) }'
}

# measure PNG: "sidebar-width icon-centre-x label-top f-top-ratio", sizes in logical px.
measure() {
  local png=$1 s side folder f
  s=$(display_scale)
  side=$(sidebar_measure "$png" | cut -d' ' -f1)
  # The first grid cell: the centre and top of the mauve folder icon right of the sidebar.
  # (Its width varies by scale: the icon theme has different art for each pixel size.)
  folder=$(magick "$png" -crop "+$(awk -v s="$s" 'BEGIN{print int(135*s)}')+$(awk -v s="$s" 'BEGIN{print int(40*s)}')" +repage \
    -fuzz 12% -fill black +opaque '#cba6f7' -fill white -opaque '#cba6f7' -trim -format '%X %Y %w' info: |
    awk -v s="$s" '{gsub(/\+/,""); printf "%d %d", ($1 + $3 / 2) / s + 135 + 0.5, $2 / s + 40 + 0.5}')
  # LOOK-20: the F of the "Folder" label under that icon, and of the sidebar's File System
  # (the 4th place: Recent, Home, Desktop, File System). Text lines land at different
  # fractions of a pixel, so checking two catches more.
  local cx top grid places
  read -r cx top <<<"$folder"
  grid=$(f_top "$png" "$(awk -v s="$s" -v x="$cx" -v y="$top" 'BEGIN{printf "%dx%d+%d+%d", 120*s, 90*s, (x-60)*s, (y+50)*s}')" 1)
  places=$(f_top "$png" "$(awk -v s="$s" -v w="$side" 'BEGIN{printf "%dx%d+%d+%d", (w-45)*s, 200*s, 40*s, 40*s}')" 4)
  # The label's first row of ink, in logical px: the cell's vertical layout. (The icon's own
  # top moves with the art.)
  local label
  label=$(magick "$png" -crop "$(awk -v s="$s" -v x="$cx" -v y="$top" 'BEGIN{printf "%dx%d+%d+%d", 120*s, 90*s, (x-60)*s, (y+50)*s}')" +repage \
    -colorspace Gray -threshold 12% -trim -format '%[fx:page.y]' info: | awk -v s="$s" -v y="$top" '{printf "%d", y + 50 + $1 / s + 0.5}')
  folder="$cx $label"
  echo "$side $folder $(awk -v a="$grid" -v b="$places" 'BEGIN{print (a < b ? a : b)}')"
}

if [ "${1:-}" = --one ]; then
  T=$(mktemp -d); trap 'cleanup; rm -rf "$T"' EXIT
  D=$T/demo; mkdir -p "$D/Folder"; touch "$D/notes.txt"; mkdir -p "$T/cfg"
  launch "$D" "$T/cfg"; sleep 1
  shot "$OUT/$2-icons.png"
  keys -M ctrl 2 -m ctrl; shot "$OUT/$2-details.png"
  measure "$OUT/$2-icons.png" > "$OUT/$2.txt"
  # LOOK-20: the font setting follows a scale change while the app runs.
  if [ "$2" = 2 ]; then
    keys -M ctrl 1 -m ctrl
    swaymsg output HEADLESS-1 scale 1 >/dev/null; sleep 1.5
    shot "$OUT/2-to-1-icons.png"
    measure "$OUT/2-to-1-icons.png" > "$OUT/2-to-1.txt"
  fi
  exit
fi

OUT=${OUT:-$(mktemp -d)}; export OUT; mkdir -p "$OUT"
for s in 2 1 1.25 1.5; do
  JD_SCALE=$s JD_RES=$(awk -v s=$s 'BEGIN{printf "%dx%d", 1600*s, 900*s}') \
    ./scripts/manual/headless.sh "$0" --one "$s" >/dev/null 2>&1
  read -r side fx fy ftop < "$OUT/$s.txt" || { echo "FAIL  scale $s: no measurements"; FAILS=$((FAILS+1)); continue; }
  echo "scale $s: sidebar ${side}px, first cell centred at ${fx}, label top ${fy}; F top bar ${ftop} of the brightest bar (the fainter of two)"
  if [ "$s" = 2 ]; then base="$side $fx $fy"; fi
  # Rounding to device px moves an edge by up to one device px, which is under 1.5
  # logical px at every scale tested.
  near() { awk -v a="$1" -v b="$2" 'BEGIN{d=a-b; exit !(d<=1.5 && d>=-1.5)}'; }
  set -- $base
  near "$side" "$1" && near "$fx" "$2" && near "$fy" "$3" && r=same || r="differs ($side $fx $fy)"
  check "scale $s: layout in logical px matches scale 2 (LOOK-19)" same "$r"
  [ "$s" = 1 ] && check "scale 1: the top of F is a solid row (LOOK-20)" yes "$(awk -v r="$ftop" 'BEGIN{print r>=0.8?"yes":"no"}')"
  if [ "$s" = 2 ]; then
    read -r _ _ _ live < "$OUT/2-to-1.txt"
    check "scale 2 changed to 1 while running: the top of F is solid (LOOK-20)" yes "$(awk -v r="$live" 'BEGIN{print r>=0.8?"yes":"no"}')"
  fi
done
echo "screenshots: $OUT"
[ "$FAILS" = 0 ] && echo "all passed" || { echo "$FAILS failed"; exit 1; }
