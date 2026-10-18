---
name: release
description: Push Junk Drawer's main branch and act on scripts/release-check.sh's recommendation, releasing a patch or minor version automatically (bump, tag, notes, push, GitHub release) and asking first only for a major one. Use when the user runs /release.
argument-hint: [none|patch|minor|major|X.Y.Z to override the recommendation]
disable-model-invocation: true
---

# /release — push, and release when the commits call for it

Override: $ARGUMENTS

The user running `/release` is the go-ahead to push `main` and, when the commits call for it, to publish a release. Don't ask again except where this skill says to. The scripts do the work: `scripts/release-check.sh` recommends a level, and `scripts/release.sh` bumps, tests, commits, tags and drafts the notes (see "Releasing" in `CLAUDE.md`).

Always push with `NO_RELEASE_CHECK=1`. The pre-push hook would otherwise offer the same release again, interactively.

## 1. Check the branch

Stop and tell the user, changing nothing, when:

- The current branch isn't `main`.
- The working tree has uncommitted changes (`git status --short`). Don't commit or stash them yourself.
- `main` is behind `origin/main` after `git fetch origin`. Say how far; don't merge or rebase.
- There is nothing to push and no release is due (`git log origin/main..main` is empty and the check below says `none`).

Then run `nix build`. It runs the tests, and a release must not ship a package that doesn't build. If it fails, stop and show the failure.

## 2. Decide the level

Run `scripts/release-check.sh`. The first line is the level (`none`, `patch`, `minor` or `major`), and the rest are the reasons.

- **An override above** (`none`, `patch`, `minor`, `major` or an exact `X.Y.Z`) wins. If it is lower than the recommendation, say so in the report, but use it.
- **`major`** without an override: ask with `AskUserQuestion`. Quote the reasons (a `BREAKING` commit, a removed config key or binding), and offer: release major (recommended), release minor instead, push without a release, or cancel.
- Otherwise use the recommendation.

Also note anything the user should know before a release, without stopping for it: rows in the roadmap's verification backlog, and rows still `planned` that the commits since the last tag mention.

## 3. Push without a release (`none`)

`NO_RELEASE_CHECK=1 git push origin main`, then report what was pushed (`git log --oneline` of the range) and why no release was needed.

## 4. Release (`patch`, `minor`, `major`, or `X.Y.Z`)

1. `scripts/release.sh LEVEL NOTES`, with `NOTES` a new file in your scratchpad. It prints the new tag (`vX.Y.Z`). If it fails, it leaves the version unchanged: stop and show why.
2. **Tidy the notes.** The draft lists the requirements that became `done` or `partial`, then the subjects of the other commits. Keep the warning and the Install section as they are. In the Changes list:
   - Keep every requirement line.
   - Keep commit lines that change what a user sees or installs, reworded as a plain sentence if the subject is terse.
   - Drop lines for work only developers see: skills, `CLAUDE.md`, spec-only edits, manual test scripts, `Reproduce …` commits.
   - If nothing is left, write one line that says what the release fixes or changes.
3. `NO_RELEASE_CHECK=1 git push origin main vX.Y.Z`.
4. `gh release create vX.Y.Z -R DisposableSoftware/junk-drawer --title "Junk Drawer X.Y.Z" -F NOTES`.

If the push fails, the release commit and tag stay local: report that and the commands to finish, and don't force anything. If `gh release create` fails after the push, show its error and the command to retry. Never delete or move a tag that was pushed.

## 5. Report

- What was pushed (the commit range), and the level with its reasons, or the override.
- For a release: the tag, the release URL that `gh` printed, and the final notes.
- Anything from step 2 the user should know: the verification backlog, `planned` rows the commits mention.
