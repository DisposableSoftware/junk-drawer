#! /usr/bin/env nix-shell
#! nix-shell -i bash -p jq
# Spec: INT-30, INT-31. Run from the repo root after `nix build`, with no Junk Drawer
# windows open. Runs --help and --version with Junk Drawer closed, then again while a
# test instance runs: no window may open and the instance must be untouched.
cd "$(dirname "$0")/../.." || exit 1
. scripts/manual/lib.sh
require_no_instance
T=$(mktemp -d); trap 'cleanup; rm -rf "$T"' EXIT
VERSION="junk $(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"

run() { # run ARGS...: prints "status first-line"
  local out status; out=$("$BIN" "$@" 2>&1); status=$?
  echo "$status $(head -1 <<<"$out")"
}
both() { # both WHEN
  check "$1: --help"           "0 Usage: junk [OPTION…] [PATH|URI…]" "$(run --help)"
  check "$1: -h after a path"  "0 Usage: junk [OPTION…] [PATH|URI…]" "$(run "$T" -h)"
  check "$1: --version"        "0 $VERSION" "$(run --version)"
  check "$1: -V"               "0 $VERSION" "$(run -V)"
}

both "closed"; sleep 1
check "closed: no window opened" 0 "$(win_count)"
launch "$T"
TITLE=$(wins | jq -r '.[0].title')
both "running"; sleep 1
check "running: still one window" 1 "$(win_count)"
check "running: window untouched" "$TITLE" "$(wins | jq -r '.[0].title')"
check "running: same process" "$PID" "$(wins | jq -r '.[0].pid')"
exit $FAILS
