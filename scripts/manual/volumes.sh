#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim wlrctl dosfstools util-linux imagemagick
# Spec: NAV-43, NAV-40, INT-10, LOOK-13. Run from the repo root after `nix build`,
# with no Junk Drawer windows open. Mounts and unmounts only a scratch FAT image attached as
# a loop device with udisksctl; nothing else is mounted or unmounted. Screenshots go
# to $OUT.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"
IMG=$T/filetest.img
truncate -s 16M "$IMG" && mkfs.vfat -n FILETEST "$IMG" >/dev/null
LOOP=$(udisksctl loop-setup -f "$IMG" --no-user-interaction | sed -n 's/.* as \(\/dev\/loop[0-9]*\)\.$/\1/p')
[ -n "$LOOP" ] || { echo "loop-setup failed" >&2; exit 2; }
# Only ever touch $LOOP, and only while it is still backed by our image.
ours() { [ "$(losetup -n -O BACK-FILE "$LOOP" 2>/dev/null | xargs)" = "$IMG" ]; }
mountpoint_of() { findmnt -rn -o TARGET -S "$LOOP" | head -1; }
teardown() {
  cleanup; kill "${BUSY:-}" 2>/dev/null
  if ours; then
    [ -n "$(mountpoint_of)" ] && udisksctl unmount -b "$LOOP" --no-user-interaction >/dev/null
    # udisks sets autoclear, so the loop device usually goes away with the unmount.
    losetup "$LOOP" &>/dev/null && udisksctl loop-delete -b "$LOOP" --no-user-interaction 2>/dev/null
  fi
  rm -rf "$T/cfg" "$T/ud" "$IMG"
}
trap teardown EXIT

mkdir -p "$T/cfg" "$T/ud/Docs"
printf 'XDG_DESKTOP_DIR="$HOME/"\nXDG_DOCUMENTS_DIR="%s"\nXDG_MUSIC_DIR="$HOME/"\nXDG_VIDEOS_DIR="$HOME/"\nXDG_DOWNLOAD_DIR="$HOME/"\nXDG_PICTURES_DIR="$HOME/"\n' "$T/ud/Docs" > "$T/cfg/user-dirs.dirs"
title() { active .title; }
point() { # point X Y: moves the pointer to window coordinates X, Y
  local x y cx cy
  read -r x y < <(wins | jq -r '.[0] | "\(.at[0]) \(.at[1])"')
  read -r cx cy < <(hyprctl cursorpos -j | jq -r '"\(.x) \(.y)"')
  wlrctl pointer move $((x + $1 + 5 - cx)) $((y + $2 + 5 - cy)); wlrctl pointer move -5 -5; sleep 0.3
}
click() { # click X Y
  [ "$(active .class)" = "$APP" ] || { echo "Junk Drawer lost focus" >&2; exit 3; }
  point "$1" "$2"; wlrctl pointer click left; sleep 1.5
}
VOL_ROW=201   # Recent, Home, Docs, File System, a line, then the volume

launch "$T/ud/Docs" "$T/cfg"
shot "$OUT/1-unmounted.png"
click 60 $VOL_ROW
MP=$(mountpoint_of)
check "clicking the volume mounts it"  yes "$([ -n "$MP" ] && echo yes || echo no)"
check "and opens it"                   "$MP" "$(title)"
point 60 $VOL_ROW; shot "$OUT/2-mounted.png"
# The eject button ends 16px from the sidebar's edge, which follows the widest row (LOOK-12).
read -r SW _ < <(sidebar_measure "$OUT/2-mounted.png"); EJECT_X=$((SW - 27))

# A process inside the volume keeps it busy: unmounting fails and the volume stays.
(cd "$MP" && exec sleep 60) & BUSY=$!
sleep 0.3; click $EJECT_X $VOL_ROW
shot "$OUT/3-busy.png"
check "a busy-volume dialog is shown"  "Volume is busy dialog" "$([ "$(active .class)" = "$APP" ] && [ "$(active .title)" = "" ] && echo "Volume is busy dialog")"
# Cancel, in the dialog's top row (GtkMountOperation's process dialog).
read -r DX DY < <(active '"\(.at[0]) \(.at[1])"')
read -r CX CY < <(hyprctl cursorpos -j | jq -r '"\(.x) \(.y)"')
wlrctl pointer move $((DX + 236 - CX)) $((DY + 105 - CY)); sleep 0.3; wlrctl pointer click left; sleep 1

check "a busy volume stays mounted"    yes "$([ -n "$(mountpoint_of)" ] && echo yes || echo no)"
check "the window left the volume first" "$HOME" "$(title)"
kill $BUSY; wait $BUSY 2>/dev/null; BUSY=
[ "$(active .class)" = "$APP" ] || hyprctl dispatch "hl.dsp.focus({ window = \"class:$APP\" })" >/dev/null

click 60 $VOL_ROW;                     check "back in the volume"  "$MP" "$(title)"
click $EJECT_X $VOL_ROW
check "eject unmounts it"              no "$([ -n "$(mountpoint_of)" ] && echo yes || echo no)"
check "the window moved to Home"       "$HOME" "$(title)"
shot "$OUT/4-after-eject.png"

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
