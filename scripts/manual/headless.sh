#! /usr/bin/env nix-shell
#! nix-shell -i bash -p sway grim jq dbus
# headless.sh SCRIPT [ARGS...]: runs a scripts/manual check in a headless sway with its
# own D-Bus session and runtime dir, so it works with the screen locked or Junk Drawer
# open. Hyprland-specific behaviour (window states, drag icons, focus rules) still needs
# a real Hyprland session. The output defaults to 3840x2160 at scale 2 (JD_RES, JD_SCALE),
# a common 4K HiDPI setup. Screenshots from `shot` go wherever the script puts them.
set -u
cd "$(dirname "$0")/../.." || exit 1
[ $# -ge 1 ] || { echo "usage: $0 scripts/manual/NAME.sh" >&2; exit 2; }
if [ -z "${JD_HEADLESS_BUS:-}" ]; then
  # A private session bus with the usual services (gvfs for volumes) but no portals,
  # which would attach to the user's real session or mount a document portal here.
  B=$(mktemp -d); mkdir "$B/services"
  for d in ${XDG_DATA_HOME:-$HOME/.local/share} ${XDG_DATA_DIRS//:/ }; do
    for f in "$d"/dbus-1/services/*.service; do
      [ -e "$f" ] && ! grep -q portal <<<"$f" && ln -sf "$f" "$B/services/"
    done
  done
  cat > "$B/bus.conf" <<CONF
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:tmpdir=$B</listen>
  <auth>EXTERNAL</auth>
  <servicedir>$B/services</servicedir>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
CONF
  # Services the bus starts inherit this environment: keep them out of the user's
  # runtime dir, and stop gvfs mounting its FUSE view there.
  mkdir -m 700 "$B/run"
  # The user's own state, config and cache stay untouched; scripts may still point
  # XDG_CONFIG_HOME or XDG_STATE_HOME at their own temp dirs. Their GTK settings (font,
  # icon theme) come along through XDG_CONFIG_DIRS, as the portal would give them.
  mkdir -p "$B/state" "$B/config" "$B/cache" "$B/xdg/gtk-4.0"
  cp "${XDG_CONFIG_HOME:-$HOME/.config}/gtk-4.0/settings.ini" "$B/xdg/gtk-4.0/" 2>/dev/null
  JD_HEADLESS_BUS=1 XDG_RUNTIME_DIR=$B/run GVFS_DISABLE_FUSE=1 \
    XDG_STATE_HOME=$B/state XDG_CONFIG_HOME=$B/config XDG_CACHE_HOME=$B/cache \
    XDG_CONFIG_DIRS=$B/xdg:${XDG_CONFIG_DIRS:-/etc/xdg} \
    dbus-run-session --config-file="$B/bus.conf" -- "$0" "$@"
  status=$?; rm -rf "$B"; exit $status
fi

R=$XDG_RUNTIME_DIR
cat > "$R/sway.conf" <<CONF
output HEADLESS-1 resolution ${JD_RES:-3840x2160} scale ${JD_SCALE:-2}
default_border none
CONF
XDG_RUNTIME_DIR=$R WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 \
  sway -c "$R/sway.conf" >"$R/sway.log" 2>&1 &
SWAY=$!
trap 'kill $SWAY 2>/dev/null; wait $SWAY 2>/dev/null' EXIT
for _ in $(seq 50); do ls "$R"/sway-ipc.*.sock >/dev/null 2>&1 && break; sleep 0.1; done
export XDG_RUNTIME_DIR=$R JD_HEADLESS=1
export WAYLAND_DISPLAY=$(cd "$R" && ls wayland-* | grep -v lock | head -1)
export SWAYSOCK=$(ls "$R"/sway-ipc.*.sock)
# Without these, an app that can't reach the headless display falls back to the user's
# session: X11 through $DISPLAY (Xwayland), or a Wayland socket it finds there.
unset DISPLAY HYPRLAND_INSTANCE_SIGNATURE
export GDK_BACKEND=wayland
if [ -z "$WAYLAND_DISPLAY" ] || [ ! -S "$R/$WAYLAND_DISPLAY" ] || ! swaymsg -t get_outputs >/dev/null 2>&1; then
  echo "headless sway didn't start; see below" >&2; tail -5 "$R/sway.log" >&2; exit 2
fi
"$@"
