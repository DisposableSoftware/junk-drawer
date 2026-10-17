#!/usr/bin/env bash
# Recommends a semantic version bump for the commits since the last v* tag.
# Prints the level (none, patch, minor or major) on the first line, then the reasons.
#   major: a commit says BREAKING, or a config key or keybinding name was removed
#   minor: a requirement became done or partial (new behaviour shipped)
#   patch: anything else that changes the package (src, data, nix, Cargo files)
#   none:  only spec, docs, scripts or tooling changed
# Usage: scripts/release-check.sh [REV]   (REV defaults to HEAD)
set -euo pipefail
cd "$(dirname "$0")/.."

rev=${1:-HEAD}
tag=$(git describe --tags --abbrev=0 --match 'v[0-9]*' "$rev" 2>/dev/null || true)
if [ -z "$tag" ]; then echo "minor"; echo "no v* tag yet"; exit 0; fi
if [ "$(git rev-list --count "$tag..$rev")" = 0 ]; then echo none; echo "nothing since $tag"; exit 0; fi

# "ID<TAB>status" for every requirement row in the spec at a revision.
requirements() {
  git ls-tree --name-only "$1" spec/ | grep -v -e README.md -e roadmap.md | while read -r f; do git show "$1:$f"; done |
    awk -F' [|] ' '/^[|] `[A-Z]+-[0-9]+` [|] / { id = $1; sub(/^[|] `/, "", id); sub(/`$/, "", id)
      st = $3; sub(/:.*/, "", st); print id "\t" st }' | sort -u
}
# First code column of a machine-checked spec table ("config" or "keybindings").
table_keys() {
  git show "$1:spec/$2" 2>/dev/null |
    awk -v t="$3" '$0 ~ "spec:" t ":begin" {on=1; next} $0 ~ "spec:" t ":end" {on=0} on && /^[|] `/' |
    awk -F'[|]' -v c="$4" '{ v = $c; gsub(/[ `]/, "", v); print v }' | sort -u
}

level=0 reasons=()
bump() { [ "$1" -gt "$level" ] && level=$1; reasons+=("$2"); }

if git log --format=%B "$tag..$rev" | grep -q 'BREAKING'; then bump 3 "a commit is marked BREAKING"; fi
while read -r k; do [ -n "$k" ] && bump 3 "config key removed: $k"; done < <(
  comm -23 <(table_keys "$tag" configuration.md config 3) <(table_keys "$rev" configuration.md config 3))
while read -r k; do [ -n "$k" ] && bump 3 "keybinding removed: $k"; done < <(
  comm -23 <(table_keys "$tag" keybindings.md keybindings 2) <(table_keys "$rev" keybindings.md keybindings 2))

while IFS=$'\t' read -r id st; do
  case $st in done | partial) ;; *) continue ;; esac
  old=$(requirements "$tag" | awk -F'\t' -v i="$id" '$1 == i { print $2 }')
  case $old in done | partial) ;; *) bump 2 "$id is now $st (was ${old:-absent})" ;; esac
done < <(requirements "$rev")

if git diff --quiet "$tag" "$rev" -- src data nix Cargo.toml Cargo.lock flake.nix; then :; else
  bump 1 "$(git diff --name-only "$tag" "$rev" -- src data nix Cargo.toml Cargo.lock flake.nix | wc -l) package files changed"
fi

names=(none patch minor major)
echo "${names[$level]}"
for r in "${reasons[@]}"; do echo "$r"; done
