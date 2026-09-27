# 16 · Branch switching

## Summary

The branch tag in the workspace header is a picker. Clicking it lists the repo's branches and checks out the chosen one in the workspace's worktree. The picker is locked while any agent in the workspace is running, and the lock explains itself.

## Why it exists

Sometimes a workspace needs to move to another branch: to continue on an existing feature branch, or to reuse a warm worktree (dependencies installed, app configured) for different work. Changing the checkout under a running agent would corrupt its work, so it's locked. Serves **Isolation is non-negotiable**: locks explain why.

## Where it lives

- In the workspace header, right of the name: `<button class="branch-tag" data-od-id="ws-branch">`.
- Menu: `<div class="popover branch-menu" id="ws-branch-menu" role="menu" aria-label="Switch branch">`.

## Anatomy

### Branch tag
- Branch icon, the branch in monospace, and a small chevron when switchable.
- Tooltip:
  - Unlocked: `Branched from main, 2 commits behind. Click to switch branch.`
  - Locked: `Branched from main, 2 commits behind. Pause or stop the running agent to switch branches.` or `…Pause or stop the 2 running agents to switch branches.`
  - The ", N commits behind" part only appears when the branch is behind.
- Hover (unlocked only): stronger background and border.

### Menu
- Label **Switch branch**.
- A scrolling list (max 280px tall, at least 280px wide) of branches, each a menu item with:
  - A check mark column (visible on the current branch).
  - The branch name in monospace, truncated with an ellipsis if long.
  - Meta on the right: `current`, `in another workspace`, `default` (main), `integration` (develop), `deploys to staging` (staging), or nothing.

### Branch list contents
In order, de-duplicated: the workspace's current branch, then the repo's branches, then branches used by other workspaces on the same repo.

## States

| State | Condition | Behaviour |
| --- | --- | --- |
| Unlocked | No thread is running (unpaused) or provisioning | Chevron shown, menu opens. |
| Locked | At least one thread is running and not paused, or provisioning | No chevron, `aria-disabled="true"`, menu doesn't exist, tooltip explains. |

Paused threads don't lock the picker, matching the "Idle" rule on Homebase cards.

### Item states
- **Current branch:** checked (`aria-checked="true"`), meta `current`. Choosing it does nothing.
- **Checked out in another workspace:** disabled, meta `in another workspace`, tooltip `Checked out in another workspace’s worktree`. Git doesn't allow the same branch in two worktrees.
- **Available:** enabled.

## Behaviour

1. Click the tag (unlocked). The menu opens and focus moves to the first enabled item.
2. Choose a branch:
   - The workspace's branch changes, and its "behind" count resets to 0.
   - A step is added to the Lead's thread: **Switched to `<branch>`** (branch icon, success tone), detail `Checked out <branch> in this worktree, previously <old branch>`.
   - Toast: `Switched <workspace name> to <branch>`.
   - The header re-renders and focus returns to the branch tag.
3. `Esc` or clicking outside closes the menu without changes; `Esc` returns focus to the tag.

The action re-checks the lock when it runs, so a branch can't be switched if an agent started in the meantime.

## Keyboard and accessibility

- Trigger: `aria-haspopup="true"`, `aria-expanded`, `aria-controls="ws-branch-menu"`. Screen-reader text `Branch` before the name; when locked, `, locked while agents are running` after it.
- Items are `role="menuitemradio"` with `aria-checked`.
- Items are Tab-reachable within the popover.

## Data model

- Reads: workspace `branch`, `base`, `behind`, `repo`, threads' `status`/`paused`; repo `branches`; other workspaces' `branch` on the same repo.
- Writes: `branch`, `behind = 0`, a timeline step.

## Simulated in the prototype

A real build runs `git switch <branch>` (or `git checkout`) in the worktree and must handle:
- **Uncommitted changes:** block with an explanation, or offer to stash / commit first. Not handled today.
- Branches that exist only on the remote (`git switch --track origin/<branch>`).
- Re-running setup if the lockfile differs between branches.
- Recomputing "behind" against the base.

## Known gaps and open questions

- **Create PR doesn't notice a switch.** If a PR was opened from the old branch, the header still says `PR #N opened` after switching.
- **The Changes panel keeps showing the old branch's diff.**
- A running app keeps running on the old code until restarted; consider prompting to restart.
- No way to create a new branch from the picker (only switch to existing ones).
- No search in the list; long branch lists rely on scrolling.
- No remote branches.
- The base branch doesn't change when switching; decide whether it should be editable too.
- The workspace name stays the same after switching, which may no longer describe the work.
