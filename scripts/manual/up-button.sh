#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim wlrctl
# Spec: VIEW-36, LOOK-11. Run from the repo root after `nix build`, with no Junk Drawer
# windows open. Clicks the up button with wlrctl. Screenshots go to $OUT: check the
# icon colour, the hover background, and the dimmed icon at /.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"; trap 'cleanup; rm -rf "$T/demo"' EXIT
D=$T/demo; mkdir -p "$D/Alpha/inner" "$D/Beta"
title() { active .title; }
point() { # point X Y: moves the pointer to window coordinates X, Y
  local x y cx cy
  read -r x y < <(wins | jq -r '.[0] | "\(.at[0]) \(.at[1])"')
  read -r cx cy < <(hyprctl cursorpos -j | jq -r '"\(.x) \(.y)"')
  # Always end with real motion: a pointer already in place gets no enter event.
  wlrctl pointer move $((x + $1 + 5 - cx)) $((y + $2 + 5 - cy)); wlrctl pointer move -5 -5; sleep 0.3
}
click_up() {
  [ "$(active .class)" = "$APP" ] || { echo "Junk Drawer lost focus" >&2; exit 3; }
  point 24 20; wlrctl pointer click left; sleep 0.6
}
crop() { # crop NAME: the top-left corner of the window
  local g; g=$(active '"\(.at[0]),\(.at[1]) 420x90"'); grim -g "$g" "$OUT/$1"
}

launch "$D"
keys -k Home -k Return; keys -k Home -k Return; check "entered Alpha/inner"       "$D/Alpha/inner" "$(title)"
point 24 20; crop 1-hover.png
click_up;                             check "up goes to Alpha"           "$D/Alpha" "$(title)"
click_up;                             check "up goes to demo"            "$D" "$(title)"
# NAV-15 selected Alpha; Return proves the view kept focus and the selection.
keys -k Return;                       check "focus stayed on the view"   "$D/Alpha" "$(title)"
point 400 300; crop 2-idle.png
cleanup; sleep 0.5

launch /
crop 3-root-disabled.png
click_up;                             check "disabled at /"              "/" "$(title)"

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
