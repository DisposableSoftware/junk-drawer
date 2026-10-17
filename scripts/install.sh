#!/bin/sh
# Installs Junk Drawer without Nix (INT-40).
#
#   scripts/install.sh                           # into ~/.local
#   sudo PREFIX=/usr/local scripts/install.sh    # for every user
#   DESTDIR=/tmp/stage PREFIX=/usr scripts/install.sh   # staged, for packagers
#   scripts/install.sh --uninstall               # same PREFIX/DESTDIR as the install
#
# Needs cargo, pkg-config and the GTK 4 development files. Never touches your config:
# Junk Drawer creates ~/.config/junk-drawer/ itself on first run.
set -eu
cd "$(dirname "$0")/.."

PREFIX=${PREFIX:-$HOME/.local}
DESTDIR=${DESTDIR:-}
ID=io.github.DisposableSoftware.JunkDrawer
ROOT=$DESTDIR$PREFIX

# source → destination under $PREFIX
FILES="
target/release/junk                      bin/junk
data/$ID.desktop                         share/applications/$ID.desktop
data/$ID.svg                             share/icons/hicolor/scalable/apps/$ID.svg
data/$ID-symbolic.svg                    share/icons/hicolor/symbolic/apps/$ID-symbolic.svg
data/$ID.metainfo.xml                    share/metainfo/$ID.metainfo.xml
data/junk.1                              share/man/man1/junk.1
data/config.example.ini                  share/doc/junk-drawer/config.example.ini
data/keybindings.example.ini             share/doc/junk-drawer/keybindings.example.ini
"

# Launchers read the desktop database and the icon cache; only refresh what's there,
# and not when staging into DESTDIR.
refresh() {
  [ -z "$DESTDIR" ] || return 0
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q "$PREFIX/share/applications" || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1 && [ -f "$PREFIX/share/icons/hicolor/index.theme" ]; then
    gtk-update-icon-cache -q -t -f "$PREFIX/share/icons/hicolor" || true
  fi
}

if [ "${1:-}" = --uninstall ]; then
  echo "$FILES" | while read -r _ dest; do
    [ -n "$dest" ] || continue
    if [ -e "$ROOT/$dest" ]; then
      rm -f "$ROOT/$dest"
      echo "removed $ROOT/$dest"
    fi
  done
  rmdir "$ROOT/share/doc/junk-drawer" 2>/dev/null || true
  refresh
  exit 0
elif [ $# -gt 0 ]; then
  echo "usage: $0 [--uninstall]" >&2
  exit 2
fi

cargo build --release --locked
echo "$FILES" | while read -r src dest; do
  [ -n "$src" ] || continue
  mode=644
  [ "$dest" = bin/junk ] && mode=755
  install -Dm"$mode" "$src" "$ROOT/$dest"
  echo "installed $ROOT/$dest"
done
refresh

case ":$PATH:" in
  *":$PREFIX/bin:"*) ;;
  *) [ -n "$DESTDIR" ] || echo "note: $PREFIX/bin is not on your PATH, so launchers can't start junk yet" ;;
esac
