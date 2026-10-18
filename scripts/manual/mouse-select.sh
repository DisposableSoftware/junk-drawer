#! /usr/bin/env nix-shell
#! nix-shell -i bash -p ydotool wtype jq grim imagemagick
# Spec: VIEW-6. Run from the repo root after `nix build`, with no Junk Drawer windows open.
# In both views: select a.txt, click empty space and check the selection is gone; with
# Ctrl or Shift held, check it stays; clicking the item's row or the column header keeps
# it. Selection is read from the screen: the accent highlight behind the item.
# Needs write access to /dev/uinput.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); export XDG_STATE_HOME=$T/state; trap 'cleanup; kill "${YD:-}" 2>/dev/null; rm -rf "$T"' EXIT
D=$T/demo; mkdir -p "$D/folder"; touch "$D/a.txt"
ydotoold >/dev/null 2>&1 & YD=$!; sleep 1
launch "$D"
read -r WX WY < <(wins | jq -r '.[0].at | "\(.[0]) \(.[1])"')
SCALE=$(hyprctl monitors -j | jq '[.[] | select(.focused)][0].scale | floor')

mv_() { hyprctl dispatch "hl.dsp.cursor.move({ x = $((WX + $1)), y = $((WY + $2)) })" >/dev/null; }
# click X Y [KEYCODE]: left click at window-relative X Y, holding an evdev key if given.
click() {
  [ "$(active .class)" = "$APP" ] || { echo "lost focus" >&2; exit 3; }
  mv_ "$1" "$2"; sleep 0.2
  [ -n "${3:-}" ] && ydotool key "$3:1"
  ydotool click 0xC0 >/dev/null
  [ -n "${3:-}" ] && ydotool key "$3:0"
  sleep 0.4
}
# selected X Y REFX REFY: "yes" if the point on the item differs from the background.
selected() {
  local png=$T/s.png
  grim -g "$WX,$WY 960x1080" "$png"
  magick "$png" -format "%[fx:abs(p{$(($1*SCALE)),$(($2*SCALE))}.r-p{$(($3*SCALE)),$(($4*SCALE))}.r)+abs(p{$(($1*SCALE)),$(($2*SCALE))}.b-p{$(($3*SCALE)),$(($4*SCALE))}.b) > 0.08 ? 1 : 0]" info: |
    sed 's/1/yes/; s/0/no/'
}
CTRL=29 SHIFT=42

# view ITEMX ITEMY SAMPLEX SAMPLEY REFX REFY EMPTYX EMPTYY [EXTRA-NAME EXTRAX EXTRAY]...
run() {
  local v=$1; shift
  local ix=$1 iy=$2 sx=$3 sy=$4 rx=$5 ry=$6 ex=$7 ey=$8; shift 8
  click "$ix" "$iy"; check "$v: clicking a.txt selects it" yes "$(selected "$sx" "$sy" "$rx" "$ry")"
  click "$ex" "$ey"; check "$v: click on empty space clears it" no "$(selected "$sx" "$sy" "$rx" "$ry")"
  click "$ix" "$iy"; click "$ex" "$ey" $CTRL
  check "$v: Ctrl+click on empty space keeps it" yes "$(selected "$sx" "$sy" "$rx" "$ry")"
  click "$ix" "$iy"; click "$ex" "$ey" $SHIFT
  check "$v: Shift+click on empty space keeps it" yes "$(selected "$sx" "$sy" "$rx" "$ry")"
  while [ $# -ge 3 ]; do
    click "$ix" "$iy"; click "$2" "$3"
    check "$v: click on $1 keeps it" yes "$(selected "$sx" "$sy" "$rx" "$ry")"
    shift 3
  done
  click "$ex" "$ey"
}

# Window-relative logical px at the default zoom, with the sidebar shown.
run icons 395 108 330 120 480 120 600 500
# VIEW-5: a rubberband from empty space (up and left over a.txt) still selects.
mv_ 500 260; sleep 0.2; ydotool click 0x40 >/dev/null; sleep 0.1
for _ in $(seq 10); do ydotool mousemove -x -8 -y -8 >/dev/null; sleep 0.05; done
ydotool click 0x80 >/dev/null; sleep 0.4
check "icons: rubberband from empty space selects a.txt" yes "$(selected 330 120 480 120)"
click 600 500
keys -M ctrl 2 -m ctrl
run details 240 138 486 138 486 300 600 600 "the row's empty part" 486 138 "the column header" 486 51
[ "$FAILS" = 0 ] || exit 1
