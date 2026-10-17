---
name: bug
description: Turn a bug report for Junk Drawer into a reproduction, a root cause, and the spec change that makes the expected behaviour explicit, then fix the code to match and verify it. Use when the user runs /bug.
argument-hint: <what went wrong, and what you expected>
disable-model-invocation: true
---

# /bug — reproduce, close the spec gap, fix

Report: $ARGUMENTS

Follow the spec-first workflow in `CLAUDE.md`. A bug means the spec failed to pin down the behaviour, or the checks failed to hold the code to it. The point of this skill is to fix both, so the same bug can't come back unnoticed.

Unlike `/feature`, there is no approval gate: once the spec delta is clear, commit it and implement it. Stop and ask only in the cases listed in step 4.

## 1. Understand the report

1. Restate the bug as **observed** vs **expected**, and note the conditions (view, flavor, config, theme, files involved).
2. Search all of `spec/` for the behaviour: the requirement rows, implementation notes, the machine-checked tables and the verification backlog. Note every row that touches it.
3. Ask with `AskUserQuestion` only if the expected behaviour is genuinely unclear and the spec doesn't settle it. Offer concrete options and put the recommended one first.

## 2. Reproduce

Reproduce the bug before changing anything, and keep the reproduction as the regression check:

- **Unit test first** when the behaviour can be tested without a display (parsing, sorting, paths, ops on a temp dir, CSS generation). Write the test and confirm it fails.
- **Otherwise a manual script** in `scripts/manual/` (source `lib.sh`), or a new check in the script that already covers the area. Follow "Manual testing" in `CLAUDE.md`: run it headless (`scripts/manual/headless.sh`) unless the bug is in compositor behaviour. On real Hyprland the guards matter most: check there are no `io.github.DisposableSoftware.JunkDrawer` clients, and check that the screen isn't locked. If the user is at the machine, ask before a test that opens a window and takes focus. Run it on an empty workspace, and go back afterwards.
- Record what you saw (test output, screenshot), so the report can show before and after.

If you can't reproduce it, say what you tried and ask the user for the missing detail. Don't guess at a fix.

## 3. Find the root cause

Trace the bug to its cause in the code. Read GTK/GIO docs or source rather than guessing (see `CLAUDE.md`). Then look for the same mistake elsewhere: other views, dialogs or code paths with the same pattern. The spec change should state the general rule, not just the one instance.

## 4. Decide the spec delta

Classify the bug against the spec:

| Case | Spec change |
|------|-------------|
| A requirement covers the behaviour and the code breaks it | The text is fine. Fix why it slipped through: tighten the row's **Verified by** to name the new regression check. If the row was `done`, it stays `done` once the fix is verified. |
| A requirement is vague or incomplete, so the bug was allowed | Reword the row so the expected behaviour is explicit and checkable. Reset its status to `planned`. |
| No requirement covers it | Add a row with the next free ID in the right prefix, with status `planned` (as `NAV-30` came from a bug). |
| The general rule belongs in more than one place | Update every affected row, or add one rule and reference it from the others. |
| A past fix failed, or a constraint isn't obvious | Add an **Implementation notes** entry saying why. |

Stop and ask with `AskUserQuestion` (quote the rows by ID and text, and give concrete options, recommended first) when:

- **The spec asks for the reported behaviour.** Then it isn't a bug against the spec. Offer: change the spec (then continue as a feature, with the user's choice as the approval), or keep it as is.
- **The fix would change a `done` requirement beyond the bug**, or conflict with another requirement, a non-goal or a stack constraint in `spec/product.md`.
- **There are several reasonable expected behaviours**, and the spec's conventions (matching Nautilus, XDG, Catppuccin) don't pick one.

Write each requirement as one checkable sentence about behaviour. Update `roadmap.md` with any `planned` rows. Re-check the draft against the whole spec for new conflicts.

## 5. Commit the spec

Commit the spec change on its own: `spec: <summary> (ID)`. In the body, give the bug in one line (observed vs expected) and the root cause. Use the usual trailers.

## 6. Fix

Implement only what the spec delta says. If the fix needs behaviour the spec doesn't describe, go back to step 4. Keep the change small, and match the surrounding code.

## 7. Verify

Follow step 3 of the workflow in `CLAUDE.md`:

- The regression check from step 2 now passes. It failed before the fix.
- `cargo clippy --all-targets -- -D warnings`, `cargo test` (spec conformance included) and `nix build` pass.
- Check every touched ID as its **Verified by** says. Re-run the manual scripts for the area, since a fix often moves something they rely on.
- Re-read the change against the requirement text, and look for behaviour the spec doesn't mention.

Then set each touched row to `done` (or `partial`, with the gap written in), fill in **Verified by** with the regression check, and update `roadmap.md`.

## 8. Commit and report

Commit the code with a `Spec: ID, ID` trailer. In the body, give the cause and the fix.

Report to the user. They didn't approve the spec beforehand, so show it here:

- **Bug and cause:** observed vs expected, and the root cause in a sentence or two.
- **Spec delta:** each changed row before and after, and each new row in full.
- **Verification:** the regression check (failed before, passes now) and the other checks run. Send screenshots of visual fixes with `SendUserFile`.
- **Anything left `partial` or unverified**, and related problems you noticed but didn't fix.
