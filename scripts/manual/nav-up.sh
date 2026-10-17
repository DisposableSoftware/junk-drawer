#! /usr/bin/env nix-shell
#! nix-shell -i bash -p wtype jq grim
# Spec: NAV-15. Run from the repo root after `nix build`, with no Junk Drawer windows open.
# Return after go-up opens the selected item, so it proves the folder came back
# selected and focused. f180 is far down the list, so the view must scroll to it.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance

T=$(mktemp -d); OUT=${OUT:-$T/shots}; mkdir -p "$OUT"; trap 'cleanup; rm -rf "$T/demo" "$T/cfg"' EXIT
D=$T/demo; for i in $(seq -w 1 200); do mkdir -p "$D/f$i"; done
mkdir -p "$T/cfg/junk-drawer"; printf '[view]\ndefault=details\n' > "$T/cfg/junk-drawer/config.ini"
title() { active .title; }

for cfg in "" "$T/cfg"; do
  name=${cfg:+details}; name=${name:-icons}; next=${cfg:+Down}; next=${next:-Right}
  launch "$D/f180" $cfg
  keys -k BackSpace;                    check "$name: go-up"                     "$D" "$(title)"
  shot "$OUT/$name.png"
  keys -k Return;                       check "$name: came-from folder selected" "$D/f180" "$(title)"
  keys -k BackSpace; keys -k "$next" -k Return
  check "$name: focus is on it ($next)"           "$D/f181" "$(title)"
  cleanup; sleep 0.5
done

echo "screenshots: $OUT"
echo; [ "$FAILS" = 0 ] && echo "all checks passed" || { echo "$FAILS check(s) failed"; exit 1; }
