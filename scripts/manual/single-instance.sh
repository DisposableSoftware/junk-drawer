#! /usr/bin/env nix-shell
#! nix-shell -i bash -p jq wtype grim
# Spec: NAV-1. Run from the repo root after `nix build`, with no Junk Drawer windows
# open. On workspace ${WS:-7}: launching twice gives two windows in one process, no
# second process stays running, and closing the last window ends the process.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance
P=$(hyprctl activeworkspace -j | jq -r .name)
trap 'cleanup; hyprctl dispatch "hl.dsp.focus({ workspace = \"$P\" })" >/dev/null' EXIT
hyprctl dispatch "hl.dsp.focus({ workspace = \"${WS:-7}\" })" >/dev/null; sleep 0.3
procs() { pgrep -xc '.junk-wrapped' || true; }
# A scratch HOME, so the windows show an empty folder rather than the user's files.
T=$(mktemp -d); export HOME=$T

launch "$T"
"$BIN" >/dev/null 2>&1 &   # no arguments, as a launcher keybinding does
for _ in $(seq 25); do [ "$(win_count)" -gt 1 ] && break; sleep 0.2; done; sleep 0.5
check "a second launch opens a second window" 2 "$(win_count)"
check "both windows are in one process"       1 "$(wins | jq '[.[].pid] | unique | length')"
check "only one junk process is running"      1 "$(procs)"

keys -k q; sleep 0.5
check "closing one window leaves the app running" "1 1" "$(win_count) $(procs)"
hyprctl dispatch "hl.dsp.focus({ window = \"class:$APP\" })" >/dev/null; sleep 0.3
keys -k q; sleep 1
check "closing the last window quits the app" "0 0" "$(win_count) $(procs)"
PID=
rm -rf "$T"
exit $FAILS
