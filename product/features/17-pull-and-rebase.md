# 17 · Pull and rebase

## Summary

When a workspace's branch falls behind its base branch, the ⋯ menu offers two ways to catch up: **Pull**, which merges the new base commits into the branch, and **Rebase branch**, which replays the branch's commits on top of the latest base. Both are disabled when the branch is up to date.

## Why it exists

Long-running agent work drifts from `main`. Catching up keeps diffs honest and PRs mergeable. Offering both merge and rebase respects different team conventions.

## Where it lives

- Workspace header ⋯ menu, the first two items (see [09](09-workspace-header.md)). Hook `ws-rebase` for Rebase.
- The palette has a Pull command per behind workspace (no Rebase command).
- "Behind" is also shown in the branch tag's tooltip: `Branched from main, 2 commits behind.`

## Anatomy

| Item | Icon | Label when behind | Label when up to date |
| --- | --- | --- | --- |
| Pull | download | `Pull <N commits> from <base>`, e.g. `Pull 2 commits from main` | `Up to date with main` (disabled) |
| Rebase | branch | `Rebase branch` (tooltip `Rebase feat/auth onto main`) | `Rebase branch` (disabled, tooltip `Already on the latest main`) |

## Behaviour

### Pull
1. Only runs when the branch is behind.
2. The behind count resets to 0.
3. Step added to the Lead's thread: **Pulled <N commits> from <base>** (download icon, success tone), detail `Merged into <branch> with no conflicts`.
4. Toast: `Pulled <N commits> from <base>`.

### Rebase
1. Only runs when the branch is behind.
2. The behind count resets to 0.
3. Step added: **Rebased <branch> onto <base>** (branch icon, success tone), detail `Replayed local commits on top of <N new commits> with no conflicts`.
4. Toast: `Rebased <branch> onto <base>`.

After either, both items become disabled and Pull reads `Up to date with main`.

## Rules and edge cases

- Neither action is locked while agents are running (unlike branch switching).
- Pluralisation is handled: `1 commit`, `2 commits`.

## Sample data

Only **Auth session timeout** starts behind (`behind: 2`, base `main`).

## Keyboard and accessibility

- Native disabled buttons with tooltips explaining the disabled state.
- The ⋯ menu follows the popover pattern: first enabled item gets focus, `Esc` closes.

## Data model

- Reads/writes workspace `behind`; reads `base`, `branch`.

## Simulated in the prototype

The "behind" count is static and only changes when these actions run. A real build must:

- Periodically `git fetch` the base and compute `git rev-list --count HEAD..origin/<base>`.
- Pull: `git merge origin/<base>` (or `git pull`) in the worktree.
- Rebase: `git rebase origin/<base>`.
- **Handle conflicts.** Every outcome today says "with no conflicts". A real build needs a conflict state: which files conflict, an option to hand the conflict to an agent, and an option to abort.
- Refuse (or stash) when the worktree has uncommitted changes, which is likely while an agent is mid-edit.
- Warn before rebasing a branch that's already been pushed or has an open PR (it will need a force push).

## Known gaps and open questions

- **Running either while an agent is editing** is risky. Decide whether they should be locked like branch switching, or pause agents automatically.
- No conflict handling.
- No indication of being *ahead* of the base (unpushed commits).
- No Push action anywhere; Create PR presumably pushes, but that isn't stated.
- No palette command for Rebase.
- Should Harness pick one default (merge or rebase) per repo in Settings and show a single "Update branch" action?
