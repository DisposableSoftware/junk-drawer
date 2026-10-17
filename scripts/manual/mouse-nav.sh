#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim wlrctl
# Spec: NAV-17. Run from the repo root after `nix build`, with no Junk Drawer windows open.
# Moves the pointer over Junk Drawer and clicks the thumb buttons with wlrctl.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); trap 'cleanup; rm -rf "$T"' EXIT
D=$T/demo; mkdir -p "$D/Alpha/inner" "$D/Beta"

launch "$D"
title() { active .title; }
thumb() { # thumb BUTTON: clicks a wlrctl button and waits for the folder to load
  [ "$(active .class)" = "$APP" ] || { echo "Junk Drawer lost focus" >&2; exit 3; }
  wlrctl pointer click "$1"; sleep 0.6
}

keys -k Home -k Return;               check "entered Alpha"            "$D/Alpha" "$(title)"
read -r X Y W H < <(wins | jq -r '.[0] | "\(.at[0]) \(.at[1]) \(.size[0]) \(.size[1])"')
read -r CX CY < <(hyprctl cursorpos -j | jq -r '"\(.x) \(.y)"')
# Over the empty part of the view, so a stray left-click would change nothing.
# End with real motion: a pointer already in place gets no enter event.
wlrctl pointer move $((X + W / 2 + 5 - CX)) $((Y + H / 2 + 5 - CY)); wlrctl pointer move -5 -5; sleep 0.3
thumb side;                           check "side (8) goes back"       "$D" "$(title)"
thumb extra;                          check "extra (9) goes forward"   "$D/Alpha" "$(title)"
thumb back;                           check "back (11) goes back"      "$D" "$(title)"
thumb forward;                        check "forward (10) goes forward" "$D/Alpha" "$(title)"
# Over Beta: the press only navigates, so nothing is selected (NAV-14) and Return
# opens nothing; had the press selected Beta, Return would open it.
wlrctl pointer move $((250 - W / 2)) $((110 - H / 2)); sleep 0.3
thumb side;                           check "side over an item"        "$D" "$(title)"
keys -k Return;                       check "the press selected nothing" "$D" "$(title)"

echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
