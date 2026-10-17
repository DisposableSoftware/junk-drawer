#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim glib dbus wl-clipboard
# Spec: NAV-5, INT-11. Run from the repo root after `nix build`, with no Junk Drawer windows
# open and nothing else owning org.freedesktop.FileManager1. Calls the service with
# gdbus while Junk Drawer runs, then checks D-Bus activation on a private bus
# (dbus-run-session) with the package's service file. Ctrl+C/wl-paste read the
# selection, which replaces the clipboard. Screenshots go to $OUT.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

NAME=org.freedesktop.FileManager1
owner() { gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
  --method org.freedesktop.DBus.GetNameOwner "$NAME" 2>/dev/null; }
[ -z "$(owner)" ] || { echo "$NAME is owned by another process: $(owner)" >&2; exit 2; }

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
trap 'cleanup; for p in $(wins | jq -r ".[].pid" | sort -u); do kill "$p"; done; rm -rf "$T/demo" "$T/share"' EXIT
D=$T/demo; mkdir -p "$D/a" "$D/b" "$D/c"; touch "$D/a/x" "$D/a/y" "$D/a/w" "$D/b/z"
fm1() { gdbus call --session --dest $NAME --object-path /org/freedesktop/FileManager1 \
  --method $NAME."$1" "$2" "" >/dev/null; sleep 1; }
titles() { wins | jq -r '[.[].title] | sort | join(" ")'; }
focus_title() { hyprctl dispatch "hl.dsp.focus({ window = \"title:^$1\$\" })" >/dev/null; sleep 0.3; }

launch "$D"
check "Junk Drawer owns the name while running" yes "$([ -n "$(owner)" ] && echo yes || echo no)"
fm1 ShowItems "['file://$D/a/x', 'file://$D/b/z', 'file://$D/a/y']"
check "ShowItems: one window per folder" "$D $D/a $D/b" "$(titles)"
focus_title "$D/a"; keys -M ctrl c -m ctrl; sleep 0.2
check "both items in a/ are selected" "file://$D/a/x file://$D/a/y" \
  "$(wl-paste -n -t text/uri-list | tr -d '\r' | sort | xargs)"
shot "$OUT/1-show-items.png"
fm1 ShowFolders "['file://$D/c']"
check "ShowFolders opens the folder" "$D $D/a $D/b $D/c" "$(titles)"
fm1 ShowItemProperties "['$D/b/z']"
check "ShowItemProperties (a plain path)" yes "$(wins | jq -r '.[].title' | grep -qx Properties && echo yes || echo no)"
focus_title Properties; shot "$OUT/2-properties.png"
cleanup; sleep 1
check "the name is released on exit" "" "$(owner)"

# Activation: a private session bus that only knows Junk Drawer's service file.
mkdir -p "$T/share/dbus-1/services"
sed "s|@bin@|$(realpath "$BIN")|" data/org.freedesktop.FileManager1.service \
  > "$T/share/dbus-1/services/$NAME.service"
XDG_DATA_DIRS=$T/share dbus-run-session -- sh -c \
  "gdbus call --session --dest $NAME --object-path /org/freedesktop/FileManager1 --method $NAME.ShowFolders \"['file://$D/c']\" '' >/dev/null; sleep 4" &
for _ in $(seq 40); do [ "$(win_count)" -gt 0 ] && break; sleep 0.1; done
sleep 1
check "activation starts Junk Drawer on the folder, without a Home window" "$D/c" "$(titles)"
wait $!

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
