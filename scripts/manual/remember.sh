#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq
# Spec: CONF-18, CONF-19, CONF-20, VIEW-3, VIEW-10, VIEW-25, NAV-20, NAV-41.
# Run from the repo root after `nix build`, with no Junk Drawer windows open. Uses a temp
# XDG_CONFIG_HOME and XDG_STATE_HOME. Changes each remembered setting, reads state.ini
# back, opens a second window and then restarts the app.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); trap 'cleanup; rm -rf "$T"' EXIT
D=$T/demo; mkdir -p "$D/a"; touch "$D/b" "$D/.hidden"; head -c 5000 /dev/zero > "$D/big"
OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
mkdir -p "$T/cfg/junk-drawer"
printf '[keybindings]\nsort-by-size = <Alt>s\nsort-reversed = <Alt>r\n' > "$T/cfg/junk-drawer/keybindings.ini"
export XDG_STATE_HOME=$T/state
S=$XDG_STATE_HOME/junk-drawer/state.ini
val() { sed -n "s/^$1=//p" "$S" 2>/dev/null; }

launch "$D" "$T/cfg"; shot "$OUT/1-first-run.png"
check "no state file before a change (CONF-18)" "" "$([ -e "$S" ] && echo yes)"
keys -M ctrl 2 -m ctrl;                       check "view saved (CONF-19)" details "$(val mode)"
keys -M ctrl h -m ctrl;                       check "hidden saved"         true "$(val show-hidden)"
keys -k F9;                                   check "sidebar saved"        false "$(val show-sidebar)"
keys -M ctrl -k equal -m ctrl;                check "details zoom saved"   64 "$(val list-icon-size)"
keys -M alt s -m alt;                         check "sort column saved"    size "$(val sort-column)"
keys -M alt r -m alt;                         check "sort direction saved" true "$(val sort-reversed)"
shot "$OUT/2-changed.png"
keys -M ctrl 1 -m ctrl -M ctrl -k equal -m ctrl
check "icon zoom saved" 168 "$(val icon-size)"
keys -M ctrl 0 -m ctrl;                       check "zoom-reset forgets it (CONF-19)" "" "$(val icon-size)"
check "config.ini untouched (CONF-18)" "[keybindings]" "$(head -1 "$T/cfg/junk-drawer/keybindings.ini")"

# A second window starts from the saved values (CONF-20).
keys -M ctrl n -m ctrl; sleep 1
shot "$OUT/3-second-window.png"
check "two windows" 2 "$(win_count)"
keys -M ctrl 2 -m ctrl
cleanup; sleep 1
launch "$D" "$T/cfg"
shot "$OUT/4-after-restart.png"
# Zoom out acts on the visible view: the details view if the saved view was restored.
keys -M ctrl -k minus -m ctrl
check "restart restores the details view (VIEW-3)" 32 "$(val list-icon-size)"
check "the icon view was not zoomed" "" "$(val icon-size)"
check "restart keeps the sort (VIEW-10)" size "$(val sort-column)"

[ "$FAILS" = 0 ] && echo "all passed" || { echo "$FAILS failed"; exit 1; }
