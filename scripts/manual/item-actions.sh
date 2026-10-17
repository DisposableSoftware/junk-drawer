#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim wlrctl
# Spec: OPS-31, OPS-34, OPS-35, OPS-36, LOOK-10. Run from the repo root after
# `nix build`, with no Junk Drawer windows open. Screenshots go to $OUT: check the menus,
# the Properties and Open With windows, and the delete confirmation in them.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"; trap 'cleanup; rm -rf "$T/demo" "$T/data" "$T/cfg"' EXIT
D=$T/demo; mkdir -p "$D/Dir/sub"; echo hello > "$D/a.txt"; echo bye > "$D/b.txt"
head -c 3000 /dev/zero > "$D/Dir/sub/blob"
mkdir -p "$T/data/applications" "$T/cfg"
cat > "$T/data/applications/recorder.desktop" <<APP
[Desktop Entry]
Type=Application
Name=Recorder
Exec=sh -c 'echo "\$1" > $T/opened' sh %f
MimeType=text/plain;
APP
printf '[Default Applications]\ntext/plain=recorder.desktop\n' > "$T/cfg/mimeapps.list"
export XDG_DATA_HOME=$T/data XDG_CONFIG_HOME=$T/cfg
title() { active .title; }
exists() { [ -e "$1" ] && echo yes || echo no; }

launch "$D"
# OPS-34: Enter accepts "Untitled Folder"; the new folder is selected and focused.
keys -M ctrl -M shift N -m shift -m ctrl;   check "new-folder opens its box"      "New Folder" "$(title)"
keys -k Return; sleep 0.5
check "folder created" "yes" "$(exists "$D/Untitled Folder")"
keys -k Return;                             check "new folder selected+focused"   "$D/Untitled Folder" "$(title)"
keys -k BackSpace
keys -M ctrl -M shift N -m shift -m ctrl; keys 'x/y' -k Return
shot "$OUT/1-slash-error.png"
check "a name with / creates nothing" "no" "$(exists "$D/x")"
keys -k Escape
keys -M ctrl -M shift N -m shift -m ctrl; keys Dir -k Return
shot "$OUT/2-exists-error.png"
keys -k Escape;                             check "back in the main window"       "$D" "$(title)"

# OPS-31: from Untitled Folder (selected by NAV-15), Right Right is b.txt.
keys -k Right -k Right -M shift -k Delete -m shift
shot "$OUT/3-delete-confirm.png"
keys -k Escape;                             check "Escape cancels delete"         "yes" "$(exists "$D/b.txt")"
keys -M shift -k Delete -m shift; keys -k Tab -k Return; sleep 0.5
check "Delete deletes" "no" "$(exists "$D/b.txt")"

# OPS-36: Properties of a.txt, then of the folder with nothing selected.
keys -k Home -k Right -k Right -M ctrl i -m ctrl; check "properties opens"              "Properties" "$(title)"
shot "$OUT/4-properties-file.png"
keys -k Escape -k Escape -M ctrl i -m ctrl; sleep 0.5
shot "$OUT/5-properties-folder.png"
keys -k Escape

# OPS-35 by keyboard: Menu on a.txt, then Open With… (second entry). The stand-in
# default app for text/plain records the files it is given.
keys -k Home -k Right -k Right -k Menu; shot "$OUT/6-item-menu.png"
keys -k Down -k Return;                     check "Open With from the menu"       "Open With" "$(title)"
shot "$OUT/7-open-with.png"
keys -k Return; sleep 1
check "the chosen app opens the file" "$D/a.txt" "$(cat "$T/opened" 2>/dev/null)"
keys -k Escape -k Menu; shot "$OUT/8-folder-menu.png"
keys -k Escape

# OPS-35 by mouse: right-click the first item (Dir), then empty space near each edge,
# where the menu must flip or slide to stay whole (no scrollbar, every entry shown).
read -r X Y W H < <(wins | jq -r '.[0] | "\(.at[0]) \(.at[1]) \(.size[0]) \(.size[1])"')
point() { # point X Y: moves the pointer to window coordinates X, Y
  local cx cy
  read -r cx cy < <(hyprctl cursorpos -j | jq -r '"\(.x) \(.y)"')
  wlrctl pointer move $((X + $1 - cx)) $((Y + $2 - cy)); sleep 0.3
}
right_click() { # right_click X Y SHOT
  point "$1" "$2"
  wlrctl pointer click right; sleep 0.5; shot "$OUT/$3"
  keys -k Escape
}
point 84 110
read -r PX PY < <(hyprctl cursorpos -j | jq -r '"\(.x) \(.y)"')
if [ "$PX" -gt "$X" ] && [ "$PY" -gt "$Y" ] && [ "$(active .class)" = "$APP" ]; then
  right_click 84 110 9-right-click-item.png
  right_click $((W - 150)) $((H - 200)) 10-right-click-bottom-right.png
  right_click 600 $((H - 60)) 11-right-click-bottom.png
  right_click $((W - 60)) 300 12-right-click-right.png
else
  echo "SKIP  right-click (pointer not over Junk Drawer)"
fi

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
