# Helpers for driving Junk Drawer on Hyprland. Source from a script run in the repo root.
# See "Manual testing on Hyprland" in CLAUDE.md for why each guard exists.
set -u

APP=io.github.DisposableSoftware.JunkDrawer
BIN=${BIN:-./result/bin/junk}

wins() { hyprctl clients -j | jq "[.[] | select(.class==\"$APP\")]"; }
win_count() { wins | jq length; }
active() { hyprctl activewindow -j | jq -r "$1"; }
focus_app() { hyprctl dispatch "hl.dsp.focus({ window = \"class:$APP\" })" >/dev/null; }

# Under scripts/manual/headless.sh the windows live in a headless sway with its own
# D-Bus session, so the user's Junk Drawer and lock screen don't matter.
if [ -n "${JD_HEADLESS:-}" ]; then
  # Same fields as hyprctl clients -j: class, pid, title, at, size, floating, focused.
  sway_wins() {
    swaymsg -t get_tree | jq "[recurse(.nodes[]?, .floating_nodes[]?) | select(.app_id != null)
      | {class: .app_id, pid, title: .name, at: [.rect.x, .rect.y],
         size: [.rect.width, .rect.height], floating: (.type == \"floating_con\"), focused}]"
  }
  wins() { sway_wins | jq "[.[] | select(.class==\"$APP\")]"; }
  active() { sway_wins | jq -r "(.[] | select(.focused)) // {} | $1"; }
  focus_app() { swaymsg "[app_id=\"$APP\"] focus" >/dev/null; }
fi

# The app is single-instance: launching while the user has Junk Drawer open would join
# their process, and killing it would close their windows.
require_no_instance() {
  if [ "$(win_count)" != 0 ]; then
    echo "Junk Drawer is already running; close it first (refusing to touch the user's windows)" >&2
    exit 2
  fi
  [ -n "${JD_HEADLESS:-}" ] && return
  if pidof hyprlock >/dev/null; then echo "screen is locked" >&2; exit 2; fi
}

# launch DIR [XDG_CONFIG_HOME]: starts Junk Drawer and focuses it. Sets PID.
launch() {
  local dir=$1 cfg=${2:-}
  [ -x "$BIN" ] || { echo "run nix build first" >&2; exit 2; }
  if [ -n "$cfg" ]; then XDG_CONFIG_HOME=$cfg "$BIN" "$dir" >"${LOG:-/dev/null}" 2>&1 &
  else "$BIN" "$dir" >"${LOG:-/dev/null}" 2>&1 & fi
  for _ in $(seq 25); do [ "$(win_count)" -gt 0 ] && break; sleep 0.2; done
  sleep 0.4
  PID=$(wins | jq -r '.[0].pid')
  focus_app
  sleep 0.3
}

# keys WTYPE-ARGS...: sends keys only if a Junk Drawer window (main or dialog) is focused.
keys() {
  if [ "$(active .class)" != "$APP" ]; then
    echo "Junk Drawer lost focus; stopping so keys don't go to another app" >&2
    cleanup; exit 3
  fi
  # Without a per-key delay wtype sends a whole sequence instantly, faster than the
  # list view processes focus moves; real typing is never that fast.
  # Headless sway drops the first key after wtype uploads its keymap; a Shift tap absorbs it.
  if [ -n "${JD_HEADLESS:-}" ]; then wtype -d 100 -k Shift_L "$@"; else wtype -d 100 "$@"; fi
  sleep 0.6
}

shot() { grim -g "$(active '"\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])"')" "$1"; }

cleanup() { [ -n "${PID:-}" ] && kill "$PID" 2>/dev/null; true; }

FAILS=0
check() { # check DESCRIPTION EXPECTED ACTUAL
  if [ "$2" = "$3" ]; then echo "PASS  $1"; else echo "FAIL  $1 (expected '$2', got '$3')"; FAILS=$((FAILS+1)); fi
}

# sidebar_measure PNG: the sidebar in a window screenshot, as "width left-pad right-pad
# line" in logical px (LOOK-12); line is 1 if a line is drawn at its edge. Needs
# imagemagick. Each column is averaged over the lower half, where there are no rows,
# so wallpaper showing through the window averages out; the edge is the biggest step.
sidebar_measure() {
  local png=$1 scale w h top profile edge line bw bx
  scale=$(hyprctl monitors -j | jq '.[] | select(.focused) | .scale')
  read -r w h < <(magick identify -format '%w %h' "$png")
  top=$((60 * ${scale%.*}))
  profile=$(magick "$png" -crop "$((w / 2))x$((h / 2 - 40))+0+$((h / 2))" +repage -colorspace Gray -scale "$((w / 2))x1!" -depth 16 txt:- | awk -F'[(,)]' 'NR>1{print $3+0}')
  read -r edge line < <(awk '{v[NR-1]=$1} END{best=0; for(i=8;i<NR-8;i++){a=(v[i-6]+v[i-5]+v[i-4]+v[i-3])/4; b=(v[i+2]+v[i+3]+v[i+4]+v[i+5])/4; d=a-b; if(d<0)d=-d; if(d>best){best=d; e=i}} spike=0; for(i=e-4;i<=e+4;i++){m=(v[i-3]+v[i+3])/2; d=v[i]-m; if(d<0)d=-d; if(d>spike)spike=d} print e, (spike>best*0.5 && spike>800)?1:0}' <<<"$profile")
  # Row contents: bright pixels (icons and names) left of the edge, below the path bar.
  read -r bw bx < <(magick "$png" -crop "${edge}x$((h - top))+0+${top}" +repage -colorspace Gray -threshold 45% -trim -format '%w %X' info:)
  bx=${bx#+}
  awk -v e="$edge" -v x="$bx" -v b="$bw" -v s="$scale" -v l="$line" 'BEGIN{printf "%d %d %d %d\n", e/s+0.5, x/s+0.5, (e-x-b)/s+0.5, l}'
}
