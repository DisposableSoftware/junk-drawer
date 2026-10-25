#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim
# Spec: NAV-30, KEY-3, NAV-12, NAV-2. Run from the repo root after `nix build`,
# with no Junk Drawer windows open.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); trap 'cleanup; rm -rf "$T"' EXIT
D=$T/demo; mkdir -p "$D/Alpha/inner" "$D/Beta"; echo x > "$D/notes.txt"
mkdir -p "$T/cfg/junk-drawer"; printf '[view]\ndefault=details\n' > "$T/cfg/junk-drawer/config.ini"
title() { active .title; }

launch "$D" "$T/cfg"
keys -k Down -k Return;                 check "Return opens folder"                "$D/Alpha" "$(title)"
keys -k BackSpace;                      check "BackSpace right after entering"     "$D" "$(title)"
keys -k Down -k Return;                 check "Return opens empty folder"          "$D/Beta" "$(title)"   # Alpha is reselected (NAV-15)
keys -k BackSpace;                      check "BackSpace from empty folder"        "$D" "$(title)"
keys -M alt -k Left -m alt;             check "Alt+Left goes back"                 "$D/Beta" "$(title)"
keys -M alt -k Right -m alt;            check "Alt+Right goes forward"             "$D" "$(title)"
keys -k Down -k Down -k Down -k F2;     check "F2 opens rename"                    "Rename" "$(title)"
keys -k BackSpace x;                    check "x/BackSpace typed in rename box"    "Rename" "$(title)"
keys -k Escape;                         check "Escape closes rename"               "$D" "$(title)"
check "rename was cancelled" "yes" "$([ -e "$D/notes.txt" ] && echo yes || echo no)"
keys -M ctrl w -m ctrl; sleep 0.3;        check "Ctrl+W closes window"                  "0" "$(win_count)"

echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
