# 08 · Teardown

## Summary

Teardown removes a workspace: it stops its agents and running app, deletes its worktree folder, and by default deletes its branch too. It's a deliberate, confirmed, destructive action with a clear warning when unmerged changes would be lost.

## Why it exists

Parallel work creates clutter: worktrees on disk, stale branches, idle agents holding memory. Teardown is how a workspace ends. It serves **Clean up is part of the job**: branch deletion is on by default so repos don't accumulate dead branches.

## Where it lives

- Workspace header ⋯ menu, last item, below a separator: **Teardown worktree…** (trash icon, danger styling).
- Opens the shared confirmation dialog (`#confirm-dialog`).

## Anatomy

### Menu item
- Label `Teardown worktree…`. The ellipsis signals a confirmation follows.
- Red text; red-tinted background on hover.

### Confirmation dialog
- **Title:** `Teardown & delete worktree?`
- **Description:** `Stops <engine name>[ and the running app], then deletes ~/.harness/worktrees/<branch with slashes replaced by dashes>. Homebase is not affected.`
  - Example: `Stops Claude Code and the running app, then deletes ~/.harness/worktrees/feat-auth. Homebase is not affected.`
  - "and the running app" only appears if the workspace's app is starting or running.
- **Checkbox:** `Also delete branch feat/auth`, **checked by default**.
- **Danger note** (only if the workspace has changed files): alert icon and `<N unmerged file changes> will be discarded. This can’t be undone.`
- **Footer:** ghost **Cancel** and a red **Teardown** button with a trash icon.

## Behaviour

1. User chooses **Teardown worktree…**. The menu closes and the dialog opens with focus on the **Teardown** button.
2. On **Teardown**:
   - The button disables and shows a spinner with `Tearing down…`.
   - After 900ms the dialog closes.
   - If **Also delete branch** is checked, the branch is removed from the repo's branch list, so it no longer appears in the base branch typeahead or the branch picker.
   - Any app timers for the workspace are cleared (stopping streaming output).
   - The workspace is removed from the list.
   - If Homebase is showing, the card fades out over about 150ms first.
   - If the torn-down workspace was open, the app navigates to Homebase; otherwise the current view re-renders.
   - Toast (danger tone): `Tore down <name> and deleted <branch>` or `Tore down <name>`.
3. On **Cancel**, `Esc`, × or backdrop click: nothing happens.

### Side effects elsewhere
- Its card, sidebar row and agent rows disappear.
- Its port is freed for other workspaces' apps.
- If it was a review workspace, the PR's button on Homebase returns to **Review in workspace**.
- Removing its repo in Settings becomes possible if it was the last workspace on that repo.

## Copy

| Element | Text |
| --- | --- |
| Menu | `Teardown worktree…` |
| Title | `Teardown & delete worktree?` |
| Checkbox | `Also delete branch <branch>` |
| Warning | `<N unmerged file changes> will be discarded. This can’t be undone.` |
| Confirm | `Teardown` / `Tearing down…` |

## Keyboard and accessibility

- Dialog is labelled by its title; focus starts on the confirm button.
- The checkbox is a native input inside its label.
- The menu item is reachable with Tab inside the popover; Esc closes the menu.

## Data model

- Reads: workspace `engine`, `branch`, `files`, `app.status`, `repo`.
- Writes: removes the workspace from `state.workspaces`; optionally removes `branch` from the repo's `branches`.
- The shared confirm dialog passes `{ deleteBranch }` to the handler, read from `#confirm-delete-branch`.

## Simulated in the prototype

A real build must, in order:

1. Stop every agent process in the workspace (graceful, then forced after a timeout).
2. Stop the app process and its children.
3. Run `git worktree remove <path>` (with `--force` only after the user has confirmed discarding changes).
4. If chosen, run `git branch -D <branch>` in the main repo. Decide whether to also delete the remote branch.
5. Surface failures (locked files, process still running, branch checked out elsewhere) and leave the workspace in place if teardown fails.

## Known gaps and open questions

- **The worktree path is shown as `~/.harness/worktrees/<branch>`** without the repo name. Two repos with the same branch name would collide. Decide the real path scheme (for example `~/.harness/worktrees/<repo>/<branch>`).
- "Unmerged file changes" counts files in the diff, not whether the branch is merged or pushed. A branch with commits pushed to an open PR has no real data loss; one with unpushed commits does. The warning should reflect pushed and merged state.
- Should branch deletion default to off when the branch has an open PR? Deleting the local branch is harmless then, but deleting the remote would close the PR.
- No undo, and no archive option (keep the transcript and summary after the worktree is gone).
- The thread transcripts are lost on teardown. Decide whether to keep them.
- **Teardown after merge** exists in Settings but no merge flow triggers it any more (see [21](21-settings.md)).
- Teardown is only available from inside the workspace. Consider adding it to Homebase cards or the palette.
