#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq wl-clipboard
# Spec: CONF-22. Run from the repo root after `nix build`, with no Junk Drawer windows open.
# Uses a temp XDG_CONFIG_HOME and XDG_STATE_HOME. A saved state.ini sorts by size; the
# first item, read back through Ctrl+C and wl-paste (which replaces the clipboard), shows
# whether it was used.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); trap 'cleanup; rm -rf "$T"' EXIT
D=$T/demo; mkdir -p "$D"
head -c 300 /dev/zero > "$D/a"; head -c 100 /dev/zero > "$D/b"; head -c 200 /dev/zero > "$D/c"
mkdir -p "$T/on/junk-drawer" "$T/off/junk-drawer" "$T/state/junk-drawer"
printf '[view]\nremember = false\n' > "$T/off/junk-drawer/config.ini"
export XDG_STATE_HOME=$T/state
S=$XDG_STATE_HOME/junk-drawer/state.ini
printf '[view]\nsort-column=size\n' > "$S"
first() { keys -k Escape -k Home -M ctrl c -m ctrl; sleep 0.2; basename "$(wl-paste -n -t text/uri-list | tr -d '\r')"; }

launch "$D" "$T/on"
check "remember = true uses state.ini (control)" b "$(first)"
cleanup; sleep 1

launch "$D" "$T/off"
check "remember = false ignores state.ini" a "$(first)"
keys -M ctrl h -m ctrl -M ctrl 2 -m ctrl -k F9 -M ctrl -k equal -m ctrl
cleanup; sleep 1
check "remember = false writes nothing" "[view] sort-column=size" "$(tr '\n' ' ' < "$S" | sed 's/ $//')"
check "no other files in the state folder" 1 "$(ls -A "$XDG_STATE_HOME/junk-drawer" | wc -l)"

[ "$FAILS" = 0 ] && echo "all passed" || { echo "$FAILS failed"; exit 1; }
