#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim
# Spec: LOOK-15, LOOK-16, CONF-1, CONF-2. Run from the repo root after `nix build`, with no
# Junk Drawer windows open. Screenshots of `flavor = gtk` go to $OUT, each with the first
# item selected, so the accent shows:
#   1-stylix.png   your own ~/.config/gtk-4.0 (Stylix: mocha shades, blue accent)
#   2-gruvbox.png  a gtk.css with gruvbox colours (brown-grey, cream text, yellow accent)
#   3-default.png  GTK_THEME=Default, which only has the older theme_* names; also
#                  3-default-details.png and 3-default-menu.png (LOOK-16: selected and
#                  hovered text is `text`, not the theme's white)
#   0-mocha.png    the default flavor, for comparison
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"; trap 'cleanup; rm -rf "$T/demo" "$T"/cfg-*' EXIT
D=$T/demo; mkdir -p "$D/Alpha" "$D/Beta"; touch "$D/notes.txt" "$D/song.mp3"
crop() { grim -g "$(active '"\(.at[0]),\(.at[1]) 900x420"')" "$OUT/$1"; }

case_() { # case_ NAME CONFIG [gtk.css text|copy] [extra]: extra also shots the details view and menu
  local cfg=$T/cfg-$1; mkdir -p "$cfg/junk-drawer" "$cfg/gtk-4.0"
  printf '[appearance]\nflavor = %s\naccent = red\n' "$2" > "$cfg/junk-drawer/config.ini"
  if [ "${3:-}" = copy ]; then cp -L ~/.config/gtk-4.0/gtk.css ~/.config/gtk-4.0/settings.ini "$cfg/gtk-4.0/"
  elif [ -n "${3:-}" ]; then printf '%s\n' "$3" > "$cfg/gtk-4.0/gtk.css"; cp -L ~/.config/gtk-4.0/settings.ini "$cfg/gtk-4.0/"; fi
  launch "$D" "$cfg"
  keys -k Home; crop "$1.png"
  if [ -n "${4:-}" ]; then
    keys -M ctrl 2 -m ctrl -k Home; crop "$1-details.png"
    keys -k Menu -k Down; shot "$OUT/$1-menu.png"; keys -k Escape
  fi
  cleanup; PID=; sleep 0.5
}

case_ 0-mocha mocha
case_ 1-stylix gtk copy
case_ 2-gruvbox gtk '@define-color window_bg_color #282828; @define-color window_fg_color #ebdbb2; @define-color accent_bg_color #d79921;'
GTK_THEME=Default case_ 3-default gtk "" extra
echo "screenshots in $OUT"
