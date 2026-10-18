#! /usr/bin/env nix-shell
#! nix-shell -i bash -p ydotool wtype jq grim imagemagick
# Spec: OPS-1, OPS-13. Run from the repo root after `nix build`, with no Junk Drawer
# windows open. Presses on each quadrant of a file icon and drags a little. A drag must
# start, and the drag image (the one near-white shape on screen) must sit where the icon
# would be had it moved with the pointer. The last case drags with no pause after the
# press: that must start a drag too, not a rubberband. Runs on workspace ${WS:-7} and
# goes back afterwards. Needs write access to /dev/uinput.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

PREV=$(hyprctl activeworkspace -j | jq -r .name)
T=$(mktemp -d); export XDG_STATE_HOME=$T/state; trap 'cleanup; kill "${YD:-}" 2>/dev/null; rm -rf "$T"; hyprctl dispatch "hl.dsp.focus({ workspace = \"$PREV\" })" >/dev/null' EXIT
hyprctl dispatch "hl.dsp.focus({ workspace = \"${WS:-7}\" })" >/dev/null; sleep 0.5
D=$T/demo; mkdir -p "$D/folder"; touch "$D/a.txt"
ydotoold >/dev/null 2>&1 & YD=$!; sleep 1
launch "$D"

mv_() { hyprctl dispatch "hl.dsp.cursor.move({ x = $1, y = $2 })" >/dev/null; }
read -r WX WY < <(wins | jq -r '.[0].at | "\(.[0]) \(.[1])"')
SCALE=$(hyprctl monitors -j | jq '[.[] | select(.focused)][0].scale | floor')
# Top-left "x y" of the drag image in window logical px, or nothing without one.
image_origin() {
  grim -g "$WX,$WY 960x1080" -t ppm - | magick - -fill black +opaque 'rgb(228,228,228)' -fill white -opaque 'rgb(228,228,228)' -trim -format '%[fx:w>20?1:0] %X %Y' info: |
    awk -v s="$SCALE" '$1 == 1 { sub(/\+/, "", $2); sub(/\+/, "", $3); printf "%d %d", $2 / s, $3 / s }'
}

# Icon rectangle of a.txt, relative to the window, at the default zoom (96 px).
X0=362 X1=428 Y0=66 Y1=150
for q in "TL $((X0+10)) $((Y0+10))" "TR $((X1-10)) $((Y0+10))" "BL $((X0+10)) $((Y1-10))" "BR $((X1-10)) $((Y1-10))" "C $(((X0+X1)/2)) $(((Y0+Y1)/2))" "C-at-once $(((X0+X1)/2)) $(((Y0+Y1)/2)) 0"; do
  set -- $q
  [ "$(active .class)" = "$APP" ] || { echo "lost focus" >&2; exit 3; }
  mv_ $((WX+540)) $((WY+600)); ydotool click 0xC0; sleep 0.4
  mv_ $((WX+$2)) $((WY+$3)); sleep 0.3
  read -r px py < <(hyprctl cursorpos | tr -d ,); ydotool click 0x40; sleep ${4:-0.2}
  for i in 1 2 3 4 5 6 7 8 9 10; do ydotool mousemove -x 6 -y 6 >/dev/null; sleep 0.06; done
  sleep 0.4
  read -r cx cy < <(hyprctl cursorpos | tr -d ,)
  origin=$(image_origin)
  wtype -k Escape; sleep 0.2; ydotool click 0x80; sleep 0.5
  check "drag starts from the $1 of the icon" yes "$([ -n "$origin" ] && echo yes || echo no)"
  [ -n "$origin" ] || continue
  read -r ix iy <<<"$origin"
  # The icon sits inside the cell at a fixed offset, so compare against the icon's
  # starting position moved by the pointer's travel.
  dx=$((ix - X0 - (cx - px))); dy=$((iy - Y0 - (cy - py)))
  check "drag image follows the grab point from the $1" ok "$([ ${dx#-} -le 4 ] && [ ${dy#-} -le 4 ] && echo ok || echo "off by $dx,$dy")"
done
[ "$FAILS" = 0 ] || exit 1
