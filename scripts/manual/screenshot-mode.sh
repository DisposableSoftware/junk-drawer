#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim
# Spec: INT-33, INT-34, NAV-45, NAV-46, OPS-38, CONF-24. Run from the repo root after `nix build`,
# headless: ./scripts/manual/headless.sh ./scripts/manual/screenshot-mode.sh
# Screenshots go to $OUT; look at them: 1-icons.png shows the sandbox (Projects, Photos, Music,
# notes.txt …, a thumbnail on holiday.png) with the dummy sidebar (Home highlighted, a Projects
# bookmark, no real volumes) in the default theme although the config says otherwise;
# 2-hidden.png adds .hidden.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
SB=/tmp/junk-screenshot
trap 'cleanup; kill "${SHOT_PID:-}" 2>/dev/null; rm -rf "$T/cfg" "$T/state" "$T/demo"' EXIT
D=$T/demo; mkdir -p "$D/Alpha"
mkdir -p "$T/cfg/junk-drawer" "$T/state/junk-drawer"
printf '[view]\nshow-sidebar=false\ndefault=details\n[appearance]\nflavor=latte\n' > "$T/cfg/junk-drawer/config.ini"
printf '[view]\nmode=details\n' > "$T/state/junk-drawer/state.ini"
echo "keep" > "$T/cfg/junk-drawer/style.css"
BEFORE=$(cd "$T" && find cfg state -type f | sort | xargs sha1sum)
title() { active .title; }
items() { ls -A "$SB" | wc -l; }

launch "$D"
NORMAL=$PID
XDG_CONFIG_HOME=$T/cfg XDG_STATE_HOME=$T/state "$BIN" --screenshot "$D" >"${LOG:-/dev/null}" 2>&1 &
for _ in $(seq 25); do [ "$(win_count)" -ge 2 ] && break; sleep 0.2; done
check "a second window opened beside the running one" 2 "$(win_count)"
SHOT_PID=$(wins | jq -r "[.[] | select(.pid != $NORMAL)][0].pid")
check "it is a separate process" 1 "$([ -n "$SHOT_PID" ] && [ "$SHOT_PID" != "$NORMAL" ] && echo 1)"
check "the running window still shows its folder" "$D" "$(wins | jq -r "[.[] | select(.pid == $NORMAL)][0].title")"
kill "$NORMAL"; PID=$SHOT_PID; sleep 0.5
focus_app; sleep 0.3
check "it shows the sandbox, not the argument" "$SB" "$(title)"
check "the sandbox has its 10 items" 10 "$(items)"
shot "$OUT/1-icons.png"

keys -k Down -k Return;    check "Return on a folder stays"  "$SB" "$(title)"
keys -k BackSpace;         check "go-up stays"               "$SB" "$(title)"
keys -k Alt_L -k Left;     check "go-back stays"             "$SB" "$(title)"
keys -k Home;              check "Home key still works"      "$SB" "$(title)"
keys -M alt -k Home -m alt; check "go-home stays"            "$SB" "$(title)"
keys -M ctrl -k l -m ctrl; keys -k Escape
keys -k Delete; keys -k Return
keys -k F2; keys -k Escape
keys -M ctrl -k c -m ctrl; keys -M ctrl -k x -m ctrl; keys -M ctrl -k v -m ctrl
keys -M ctrl -M shift -k n -m shift -m ctrl; keys -k Escape
check "the sandbox is unchanged after Delete, F2, Ctrl+C/X/V and New Folder" 10 "$(items)"
check "no new folder"  0 "$(ls "$SB" | grep -c 'Untitled')"
keys -M ctrl -k h -m ctrl; shot "$OUT/2-hidden.png"
check "settings: no file written or created" "$BEFORE" "$(cd "$T" && find cfg state -type f | sort | xargs sha1sum)"

keys -M ctrl -k w -m ctrl; sleep 1
check "the process ended with its window" gone "$(kill -0 "$SHOT_PID" 2>/dev/null && echo alive || echo gone)"
check "the sandbox is gone" "" "$([ -e "$SB" ] && echo yes)"

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
