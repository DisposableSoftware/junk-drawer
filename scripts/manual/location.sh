#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim wl-clipboard
# Spec: NAV-16. Run from the repo root after `nix build`, with no Junk Drawer windows open.
# Uses Ctrl+C and wl-paste to see which file got selected (this replaces the
# clipboard). Screenshots go to $OUT.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"; trap 'cleanup; rm -rf "$T/demo" "$T/cfg"' EXIT
D=$T/demo; mkdir -p "$D/Alpha/inner" "$D/Beta" "$D/Bravo"; echo hi > "$D/note.txt"
mkdir -p "$T/cfg/junk-drawer"; printf '[view]\nshow-path=false\n' > "$T/cfg/junk-drawer/config.ini"
title() { active .title; }
go() { keys -M ctrl l -m ctrl; keys "$1" -k Return; sleep 0.4; }

launch "$D"
keys -M ctrl l -m ctrl; shot "$OUT/1-entry.png"; keys -k Escape
go Alpha;                              check "a relative path"               "$D/Alpha" "$(title)"
go '~';                                check "~ is home"                     "$HOME" "$(title)"
go "$D/../demo/Beta";                  check "an absolute path with .."      "$D/Beta" "$(title)"
go "file://$D";                        check "a URI"                         "$D" "$(title)"

keys -M ctrl l -m ctrl; keys "$D/nothing-here" -k Return; sleep 0.5
shot "$OUT/2-missing.png"
check "a missing location shows an error" "" "$(title)"
keys -k Return; sleep 0.3              # Close
keys -k BackSpace -k BackSpace -k BackSpace -k BackSpace -k BackSpace -k BackSpace -k BackSpace -k BackSpace -k BackSpace -k BackSpace -k BackSpace -k BackSpace "Alpha" -k Return; sleep 0.4
check "the entry stayed open with the text" "$D/Alpha" "$(title)"

keys -M ctrl l -m ctrl; keys "$D/Al" -k Tab; sleep 0.4; keys "inn" -k Tab; sleep 0.4; keys -k Return; sleep 0.4
check "Tab completes a unique folder"  "$D/Alpha/inner" "$(title)"
keys -M ctrl l -m ctrl; keys "$D/B" -k Tab; sleep 0.4; keys "et" -k Tab; sleep 0.4; keys -k Return; sleep 0.4
check "Tab stops at a shared prefix"   "$D/Beta" "$(title)"

go "$D/note.txt";                      check "a file opens its folder"       "$D" "$(title)"
keys -M ctrl c -m ctrl; sleep 0.3
check "and selects the file" "file://$D/note.txt" "$(wl-paste -n -t text/uri-list | tr -d '\r')"

keys -M ctrl l -m ctrl; keys -k Escape; keys -k Home -k Return
check "Escape returns focus to the view" "$D/Alpha" "$(title)"
cleanup; sleep 0.5

launch "$D" "$T/cfg"
keys -M ctrl l -m ctrl; shot "$OUT/3-hidden-bar-open.png"
keys -k Escape; shot "$OUT/4-hidden-bar-closed.png"

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
