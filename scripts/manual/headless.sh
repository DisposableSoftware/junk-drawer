#! /usr/bin/env nix-shell
#! nix-shell -i bash -p sway grim jq dbus
# headless.sh SCRIPT [ARGS...]: runs a scripts/manual check in a headless sway with its
# own D-Bus session and runtime dir, so it works with the screen locked or Junk Drawer
# open. Hyprland-specific behaviour (window states, drag icons, focus rules) still needs
# a real Hyprland session. The output defaults to 3840x2160 at scale 2 (JD_RES, JD_SCALE),
# like bayle's monitor. Screenshots from `shot` go wherever the script puts them.
set -u
cd "$(dirname "$0")/../.." || exit 1
[ $# -ge 1 ] || { echo "usage: $0 scripts/manual/NAME.sh" >&2; exit 2; }
if [ -z "${JD_HEADLESS_BUS:-}" ]; then
  exec env JD_HEADLESS_BUS=1 dbus-run-session -- "$0" "$@"
fi

R=$(mktemp -d); chmod 700 "$R"
cat > "$R/sway.conf" <<CONF
output HEADLESS-1 resolution ${JD_RES:-3840x2160} scale ${JD_SCALE:-2}
default_border none
CONF
XDG_RUNTIME_DIR=$R WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 \
  sway -c "$R/sway.conf" >"$R/sway.log" 2>&1 &
SWAY=$!
trap 'kill $SWAY 2>/dev/null; wait $SWAY 2>/dev/null; rm -rf "$R"' EXIT
for _ in $(seq 50); do ls "$R"/sway-ipc.*.sock >/dev/null 2>&1 && break; sleep 0.1; done
export XDG_RUNTIME_DIR=$R JD_HEADLESS=1
export WAYLAND_DISPLAY=$(cd "$R" && ls wayland-* | grep -v lock | head -1)
export SWAYSOCK=$(ls "$R"/sway-ipc.*.sock)
"$@"
