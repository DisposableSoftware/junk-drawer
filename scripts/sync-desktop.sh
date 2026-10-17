#! /usr/bin/env nix-shell
#! nix-shell -i bash -p desktop-file-utils gtk3
# For development: makes launchers show the dev build in ./result without installing
# it. Copies the desktop entry (with Exec pointing at ./result/bin/junk) and both icons
# into ~/.local/share, but only the files that changed, then refreshes the desktop
# database and icon cache. Run it after `nix build`.
#
#   scripts/sync-desktop.sh            # copy what changed
#   scripts/sync-desktop.sh --remove   # take it all out again
set -eu
cd "$(dirname "$0")/.."

ID=io.github.DisposableSoftware.JunkDrawer
DATA=${XDG_DATA_HOME:-$HOME/.local/share}
RESULT=$PWD/result
APPS=$DATA/applications
ICONS=$DATA/icons/hicolor
TARGETS="$APPS/$ID.desktop $ICONS/scalable/apps/$ID.svg $ICONS/symbolic/apps/$ID-symbolic.svg"

refresh() {
  update-desktop-database -q "$APPS" || true
  gtk-update-icon-cache -q -t -f "$ICONS" || true
}

if [ "${1:-}" = --remove ]; then
  for f in $TARGETS; do
    [ -e "$f" ] || [ -L "$f" ] || continue
    rm -f "$f"
    echo "removed $f"
  done
  refresh
  exit 0
elif [ $# -gt 0 ]; then
  echo "usage: $0 [--remove]" >&2
  exit 2
fi

[ -x "$RESULT/bin/junk" ] || { echo "no dev build: run nix build first" >&2; exit 1; }

changed=0
# sync SRC DEST: replaces DEST (a file or an old symlink) when its contents differ.
sync() {
  if [ -L "$2" ] || ! cmp -s "$1" "$2"; then
    mkdir -p "$(dirname "$2")"
    rm -f "$2"
    cp "$1" "$2"
    chmod 644 "$2"
    echo "updated $2"
    changed=1
  fi
}

T=$(mktemp -d)/$ID.desktop
trap 'rm -rf "$(dirname "$T")"' EXIT
sed "s#^Exec=junk#Exec=$RESULT/bin/junk#" "$RESULT/share/applications/$ID.desktop" > "$T"
desktop-file-validate "$T"
sync "$T" "$APPS/$ID.desktop"
sync "$RESULT/share/icons/hicolor/scalable/apps/$ID.svg" "$ICONS/scalable/apps/$ID.svg"
sync "$RESULT/share/icons/hicolor/symbolic/apps/$ID-symbolic.svg" "$ICONS/symbolic/apps/$ID-symbolic.svg"

if [ "$changed" = 1 ]; then
  refresh
  echo "launchers that cache icons (docks, window switchers) may need a restart"
else
  echo "already up to date"
fi
