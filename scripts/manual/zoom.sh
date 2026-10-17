#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim wlrctl
# Spec: VIEW-25 (and KEY-10 with the zoom bindings). Run from the repo root after
# `nix build`, with no Junk Drawer windows open. Look at the screenshots in $OUT: the file
# names say which icon size each one should show.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"; trap 'cleanup; rm -rf "$T/demo"' EXIT
D=$T/demo; mkdir -p "$D/Alpha" "$D/Beta" "$D/Gamma"; echo x > "$D/notes.txt"

launch "$D"
shot "$OUT/grid-1-96.png"
keys -M ctrl -k equal -m ctrl;  shot "$OUT/grid-2-168.png"
keys -M ctrl -k plus -m ctrl;   shot "$OUT/grid-3-256.png"
keys -M ctrl -k equal -m ctrl;  shot "$OUT/grid-4-256-max.png"
keys -M ctrl 0 -m ctrl;      shot "$OUT/grid-5-96-reset.png"
keys -M ctrl -k minus -m ctrl -M ctrl -k minus -m ctrl -M ctrl -k minus -m ctrl
shot "$OUT/grid-6-48-min.png"
keys -M ctrl 2 -m ctrl;      shot "$OUT/list-1-32.png"
keys -M ctrl -k minus -m ctrl;  shot "$OUT/list-2-16.png"
keys -M ctrl 1 -m ctrl;      shot "$OUT/grid-7-48-kept.png"
# Two Rights select Beta (the first only focuses Alpha). Right after zooming must
# continue from Beta; with focus lost it would land on Alpha.
keys -k Right -k Right; keys -M ctrl -k equal -m ctrl; keys -k Right -k Return
check "focus stays on the selected item" "$D/Gamma" "$(active .title)"
keys -k BackSpace

# Ctrl+scroll up over the view: move the pointer in and hold Ctrl while scrolling.
read -r X Y W H < <(wins | jq -r '.[0] | "\(.at[0]) \(.at[1]) \(.size[0]) \(.size[1])"')
read -r CX CY < <(hyprctl cursorpos -j | jq -r '"\(.x) \(.y)"')
wlrctl pointer move $((X + W / 2 - CX)) $((Y + H / 2 - CY)); sleep 0.3
if [ "$(active .class)" = "$APP" ]; then
  wtype -M ctrl -s 1200 -m ctrl & sleep 0.4; wlrctl pointer scroll -10 0; wait $!; sleep 0.5
  shot "$OUT/grid-8-64-scrolled.png"
else
  echo "SKIP  Ctrl+scroll (pointer not over Junk Drawer)"
fi
keys '?'; shot "$OUT/help.png"

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
