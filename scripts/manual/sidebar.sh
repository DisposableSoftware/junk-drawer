#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim wlrctl
# Spec: NAV-40, NAV-41, NAV-42, INT-8, INT-9, LOOK-12, CONF-15. Run from the repo
# root after `nix build`, with no Junk Drawer windows open. Uses a temp XDG_CONFIG_HOME, so
# the bookmarks file is a scratch copy. Row positions assume the default font and the
# user's six XDG dirs; screenshots go to $OUT.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
trap 'cleanup; rm -rf "$T/demo" "$T/cfg" "$T/cfg2" "$T/ud"' EXIT
D=$T/demo; mkdir -p "$D/Alpha" "$D/Beta" "$D/Gamma"
mkdir -p "$T/cfg/gtk-3.0" "$T/cfg2/junk-drawer"
B=$T/cfg/gtk-3.0/bookmarks
printf 'file://%s Bm Alpha\n' "$D/Alpha" > "$B"
printf '[view]\nshow-sidebar=false\n' > "$T/cfg2/junk-drawer/config.ini"
# INT-8: Documents exists, Music is missing and Videos is disabled (= $HOME); the
# rest fall back to glib defaults, so they are disabled too.
mkdir -p "$T/ud/Docs"
for c in "$T/cfg" "$T/cfg2"; do
  printf 'XDG_DESKTOP_DIR="$HOME/"\nXDG_DOCUMENTS_DIR="%s"\nXDG_MUSIC_DIR="%s"\nXDG_VIDEOS_DIR="$HOME/"\nXDG_DOWNLOAD_DIR="$HOME/"\nXDG_PICTURES_DIR="$HOME/"\n' "$T/ud/Docs" "$T/ud/missing" > "$c/user-dirs.dirs"
done
title() { active .title; }
point() { # point X Y: moves the pointer to window coordinates X, Y
  local x y cx cy
  read -r x y < <(wins | jq -r '.[0] | "\(.at[0]) \(.at[1])"')
  read -r cx cy < <(hyprctl cursorpos -j | jq -r '"\(.x) \(.y)"')
  wlrctl pointer move $((x + $1 + 5 - cx)) $((y + $2 + 5 - cy)); wlrctl pointer move -5 -5; sleep 0.3
}
click() { # click X Y [BUTTON]
  [ "$(active .class)" = "$APP" ] || { echo "Junk Drawer lost focus" >&2; cleanup; exit 3; }
  point "$1" "$2"; wlrctl pointer click "${3:-left}"; sleep 0.7
}
HOME_ROW=56 DOCS_ROW=88 FS_ROW=120 BM1=165 BM2=197   # Recent is the first row

launch "$D" "$T/cfg"
shot "$OUT/1-sidebar.png"
click 60 $HOME_ROW;                   check "Home row opens home"          "$HOME" "$(title)"
click 60 $DOCS_ROW;                   check "user dir row (INT-8)"         "$T/ud/Docs" "$(title)"
click 60 $FS_ROW;                     check "File System row opens /"      "/" "$(title)"
keys -k Home -k Return
[ "$(title)" != / ] && r=moved || r=stuck; check "focus stayed on the view" moved "$r"
click 60 $BM1;                        check "bookmark row opens its folder" "$D/Alpha" "$(title)"
shot "$OUT/2-current-bookmark.png"

keys -M ctrl d -m ctrl; sleep 0.5;    check "Ctrl+D on a bookmark adds nothing" 1 "$(wc -l < "$B")"
keys -k BackSpace -k Right -k Return; check "in Beta"                      "$D/Beta" "$(title)"
keys -M ctrl d -m ctrl; sleep 0.8;    check "Ctrl+D adds the folder"       "file://$D/Beta" "$(tail -1 "$B")"
shot "$OUT/3-added.png"
click 60 $BM2 right; shot "$OUT/4-remove-menu.png"
click 140 $((BM2 + 24)); sleep 0.5;  check "Remove Bookmark removes it"   1 "$(wc -l < "$B")"
printf 'file://%s Gm\n' "$D/Gamma" >> "$B"; sleep 1
click 60 $BM2;                        check "an outside edit shows up"     "$D/Gamma" "$(title)"

keys -k F9; shot "$OUT/5-hidden.png"
click 60 $HOME_ROW;                   check "hidden sidebar can't be clicked" "$D/Gamma" "$(title)"
keys -k F9; click 60 $HOME_ROW;       check "F9 shows it again"            "$HOME" "$(title)"
cleanup; sleep 0.5

launch "$D" "$T/cfg2"
shot "$OUT/6-show-sidebar-false.png"
keys -k F9; shot "$OUT/7-f9-shows.png"

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
