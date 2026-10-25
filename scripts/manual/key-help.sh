#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim wlrctl
# Spec: KEY-8, KEY-9, KEY-10, LOOK-8. Run from the repo root after `nix build`, with no
# Junk Drawer windows open. Screenshots go to $OUT (default: a temp dir); look at them to check
# the overlay's rows, colours and that nothing is cut off.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
trap 'cleanup; rm -rf "$T/demo" "$T/cfg"' EXIT
D=$T/demo; mkdir -p "$D/Alpha"
mkdir -p "$T/cfg/junk-drawer"; printf '[keybindings]\ngo-home=<Alt>h\ntrash=\n' > "$T/cfg/junk-drawer/config.ini"
title() { active .title; }

launch "$D/Alpha"
keys '?'
shot "$OUT/1-help.png"
keys x;                                 check "a key dismisses without acting" "1" "$(win_count)"
shot "$OUT/2-dismissed.png"
keys -k BackSpace;                      check "focus is back on the view"       "$D" "$(title)"

keys '?' -k Shift_L -k Control_L x;     check "modifiers alone don't dismiss"   "1" "$(win_count)"
keys '?' -M ctrl w -m ctrl; sleep 0.3;  check "Ctrl+W is swallowed"            "1" "$(win_count)"

# wlrctl moves the pointer relatively; only click if it really lands inside Junk Drawer.
keys '?'
read -r X Y W H < <(wins | jq -r '.[0] | "\(.at[0]) \(.at[1]) \(.size[0]) \(.size[1])"')
read -r CX CY < <(hyprctl cursorpos -j | jq -r '"\(.x) \(.y)"')
wlrctl pointer move $((X + W / 2 - CX)) $((Y + H / 2 - CY)); sleep 0.3
read -r PX PY < <(hyprctl cursorpos -j | jq -r '"\(.x) \(.y)"')
if [ "$PX" -gt "$X" ] && [ "$PX" -lt $((X + W)) ] && [ "$PY" -gt "$Y" ] && [ "$PY" -lt $((Y + H)) ] \
  && [ "$(active .class)" = "$APP" ]; then
  wlrctl pointer click left; sleep 0.5
  keys -M ctrl w -m ctrl; sleep 0.3;      check "click dismisses (then Ctrl+W closes)" "0" "$(win_count)"
else
  echo "SKIP  click dismisses (pointer not over Junk Drawer)"
fi
cleanup; sleep 0.5

launch "$D" "$T/cfg"
keys '?'
shot "$OUT/3-overridden.png"
cleanup; sleep 0.5

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
