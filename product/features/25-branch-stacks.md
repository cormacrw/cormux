# 25 · Branch stacks

## Summary

The left pane of the **Git** tab (14) shows the workspace's stack of branches and pull requests. The stack belongs to GitHub's [`gh stack`](https://github.com/github/gh-stack) CLI extension, so it matches what `gh stack view` prints and what github.com shows. Uncommitted changes sit on top as their own level. Clicking a level shows its diff on the right. Each branch is a card with its line counts and commit count against the branch below it, its pull request, and a **Check out** button. **Add branch**, **Push** and **Sync** run the matching `gh stack` commands. Pull requests are always opened by hand with **Create PR**, which works exactly as before; nothing in the Stack tab creates or edits one.

## Why it exists

Big changes review better as a series of small pull requests, each built on the last. gh-stack keeps the chain of branches straight: it creates branches on top of each other, rebases them together, and pushes them together. The Stack tab puts that inside the workspace instead of a separate terminal.

## Where it lives

- `<nav aria-label="Stack">`, the left pane of the Git tabpanel (`#changes-panel`). There is no separate Stack tab.
- Without a stack (not stacked, or gh-stack unavailable) it still renders as one: uncommitted changes, the checked-out branch against the trunk, then the trunk.

## States

| State | Condition | Panel |
| --- | --- | --- |
| Stacked | `gh stack view --json` succeeds | Cards, trunk, **Sync**, **Add branch**, **Push** (primary) |
| Not stacked | Exit code 2 | The current branch marked `not stacked`, and **Add branch** (primary) to start a stack |
| Unavailable | `gh` or the extension is missing, or exit code 9 (stacks off for the repo) | Why, plus the install command |

## Anatomy

- Heading **Stack** with a subtitle, e.g. `3 open branches stacked on main`.
- **Uncommitted changes** always first, above the top branch. It's the default diff.
- Cards, top of the stack first, joined by a rail, then the trunk. Clicking a card shows that branch's committed changes against the branch below it (the trunk for the bottom branch) as `git diff <base>...<branch>`, whichever branch is checked out. Each card shows:
  - The branch in monospace, a `current` badge on the checked-out branch, and `needs rebase` when the branch below (the trunk for the bottom branch, `origin/<trunk>` when it exists) has commits this branch doesn't: gh-stack's flag, or `git merge-base --is-ancestor <base> <branch>` failing. An unstacked branch gets the same badge against the trunk (`currentNeedsRebase`), pointing at **Rebase branch** (17) instead of Sync.
  - `+adds −deletes · N commits` against the nearest unmerged branch below (or the trunk). Merged branches are dimmed and say `Merged into main`.
  - `#N` when the branch has a pull request, from gh-stack or, before gh-stack has seen it, from the synced open PRs (matched by head branch). Clicking it opens the PR.
  - A **Check out** icon button, except on the current branch and merged branches. Merged branches can't be selected.

## Behaviour

### Check out
A plain `git switch`, the same as the branch picker (16). Disabled while an agent is running or the workspace is provisioning, and for branches checked out in another workspace.

### Add branch
gh-stack only adds branches on top, so the button is disabled unless the top branch is checked out, and its tooltip says which branch to check out.
1. The inline form reads `New branch on top of <current branch>`. The name is checked like a new workspace branch.
2. **Create** or `Enter`:
   - Stacked: `gh stack add <name>`.
   - Not stacked, on the trunk: `gh stack init --base <trunk> <name>`, so the new branch is the bottom.
   - Not stacked, on another branch: `gh stack init --base <trunk> <current>` adopts it as the bottom, then `gh stack add <name>`.
3. The workspace follows the new branch (Lead step, toast). Uncommitted changes move with it.

### Push
`gh stack push --remote origin`: pushes every unmerged branch in the stack, each with `--force-with-lease`, so rebased branches update without overwriting someone else's work. It never opens or edits a pull request. Toast: `Pushed the stack`.

### Sync
`gh stack sync --remote origin`: fetches, rebases the stack onto the trunk, pushes, and refreshes what gh-stack knows about each branch's PR (read-only; it doesn't create or edit PRs). Locked while agents run, since it rewrites the checked-out branch. Errors:
- Rebase conflict (exit 3): gh-stack restores the stack; the toast says to run `gh stack rebase` in the worktree.
- Divergence between local and GitHub: with no terminal gh-stack makes no changes; the toast says to run `gh stack sync` in a terminal to choose.

### Create PR
Still opened by hand from the header, one branch at a time, with the same dialog as 18. Two things follow the stack:
- **Base:** a stacked branch's PR targets the nearest unmerged branch below it; anything else targets the workspace's base branch as before. The dialog's route line shows that base (`feat/oauth-login → feat/oauth-api · 3 files +96 −12`), and the drafted reason and "What changed" are measured against it. If that branch isn't on `origin` yet, Create PR asks you to Push the stack first.
- **Existing PRs:** the header shows the checked-out branch's PR instead of **Create PR** when there is one: the PR this workspace opened, else a synced open PR with this head branch, else the PR gh-stack knows about. Clicking it opens the PR.

## Technical notes

- Commands run with piped stdio and `GH_PROMPT_DISABLED=1`, plus the flags gh-stack documents for non-interactive use, in the worktree's per-worktree git lock.
- `gh`'s own login is used. Cormux's Keychain token is passed as `GH_TOKEN` only when `gh` isn't signed in.
- gh-stack keeps its state in `$(git rev-parse --git-dir)/gh-stack`, which is **per worktree**. A stack started in your main checkout isn't visible in a workspace until you run `gh stack checkout <branch or PR>` there.
- Exit codes follow gh-stack v0.1: 2 not in a stack, 3 rebase conflict, 4 GitHub API, 8 locked, 9 stacks unavailable.
- Commands: `get_workspace_stack`, `add_stack_branch`, `push_stack`, `sync_stack`. Checkout reuses `switch_workspace_branch`.

## Known gaps and open questions

- No merge button (`gh stack merge <pr> --yes`), no `--prune` on sync, and no rebase-conflict flow beyond the toast.
- No way to reorder or remove branches: `gh stack modify` is TUI-only.
- No UI to pull a stack from GitHub into a workspace (`gh stack checkout <pr>`).
- Cormux doesn't retarget a PR when the branch below it merges. Check its base on GitHub afterwards.
- Counts refresh when the Git tab opens, the branch changes, or after Push/Sync, not when an agent commits while the tab is open.
