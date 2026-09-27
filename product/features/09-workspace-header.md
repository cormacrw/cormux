# 09 · Workspace header

## Summary

The workspace header is a single, clean bar across the top of a workspace. On the left: the workspace name and its branch. On the right: the workspace's actions, in a fixed order: app run controls, the Changes toggle, a ⋯ menu for less frequent actions, and one primary action (**Create PR**, or **Submit review** in review workspaces).

## Why it exists

The header was deliberately stripped back. It previously held a back button, a status badge, a repo line and a large red Teardown button. Now it carries identity (name and branch) and actions only. Status lives in the tabs and sidebar; navigation lives in the sidebar.

## Where it lives

- `<header class="ws-top" id="ws-top" data-od-id="ws-header">` at the top of the workspace view.
- Hooks: `ws-branch`, `ws-actions`, `ws-run-controls`, `ws-output-toggle`, `ws-run`, `ws-restart`, `ws-stop`, `ws-changes-toggle`, `ws-more`, `ws-rebase`, `ws-create-pr`, `ws-submit-review`.

## Anatomy

### Left: identity

1. **Workspace name** (`h1`, regular sans font, not monospace). Truncates with an ellipsis; the full name is in its tooltip. Receives focus when the workspace opens.
2. **Branch tag / picker**: a small bordered tag with a branch icon, the branch in monospace, and a chevron. Clicking it opens the branch picker. Locked while agents are running. Full detail in [16](16-branch-switching.md).
   - Tooltip always includes `Branched from <base>[, <N commits> behind].`

### Right: actions, in order

1. **Run controls**: one joined control for the app. Full detail in [15](15-run-app-and-output.md).
   - Left segment toggles the Output tab and shows app state: `Output` (stopped), a spinner and `Starting…`, or a green dot and `localhost:5173`.
   - Right segment: **Run** (stopped) or **Restart** and **Stop** icon buttons (starting or running).
2. **Changes** secondary button with a file icon. When the workspace has changed files it also shows `+N −N`. Toggles the Changes panel (`aria-pressed`). Full detail in [14](14-changes-panel.md).
3. **⋯ (More workspace actions)** bordered icon button, opening a right-aligned menu:
   | Item | Icon | Enabled when | Detail |
   | --- | --- | --- | --- |
   | `Pull <N commits> from <base>` / `Up to date with <base>` | download | Branch is behind | See [17](17-pull-and-rebase.md) |
   | `Rebase branch` | branch | Branch is behind | Tooltip `Rebase <branch> onto <base>` or `Already on the latest <base>` |
   | `New thread` | plus | Always | See [10](10-thread-tabs.md) |
   | separator | | | |
   | `Teardown worktree…` | trash, red | Always | See [08](08-teardown.md) |
4. **Primary action** (solid, the only primary in the header):
   - Build workspace: **Create PR** with a PR icon. After a PR is created it becomes a disabled `PR #<N> opened`. See [18](18-create-pr.md).
   - Review workspace: **Submit review** with a PR icon. Disabled after submitting. See [19](19-review-workspaces.md).

## Behaviour

### Popovers (⋯ menu and branch picker)
- Clicking the trigger opens the popover, sets `aria-expanded="true"` and focuses its first enabled item.
- Clicking the trigger again, clicking anywhere outside, or pressing `Esc` closes it. `Esc` returns focus to the trigger.
- Only one popover is open at a time.
- Choosing an item closes the popover and runs the action.

### Re-rendering
The header is rebuilt on every state change so labels, counts and enabled states are always current. After a rebuild, focus is restored to the same control if it still exists.

## States summary

| Situation | Visible differences |
| --- | --- |
| Provisioning | Branch picker locked; Run disabled with tooltip `Available once the worktree is set up`. |
| Agents running | Branch picker locked, no chevron. |
| Branch behind base | Pull and Rebase enabled in ⋯. |
| Up to date | Pull reads `Up to date with main` and is disabled; Rebase disabled. |
| No changed files | Changes button shows no counts. |
| PR created | Primary reads `PR #N opened`, disabled. |
| Review workspace | Primary is **Submit review**. |
| Review submitted | **Submit review** disabled. |

## Keyboard and accessibility

- Header is a `<header>` inside the workspace `section` (labelled "Workspace").
- The action group is a plain container; run controls are a `role="group"` labelled "App".
- ⋯ trigger: `aria-haspopup="true"`, `aria-controls="ws-more"`, `aria-label="More workspace actions"`.
- Disabled menu items use the native `disabled` attribute and keep their tooltips.

## Data model

Reads: `title`, `branch`, `base`, `behind`, `kind`, `prNum`, `activity`, `files`, threads' `status`/`paused`, and the workspace's app state.

## Known gaps and open questions

- On narrow windows the right-hand group can crowd the name. There's no overflow strategy beyond truncating the name; consider collapsing Changes and the run control into the ⋯ menu below a breakpoint.
- There's no **Merge** action any more. Create PR replaced it. Decide whether direct merge (for solo repos) returns, perhaps in the ⋯ menu.
- No "Open in editor" or "Reveal in Finder" for the worktree, which developers will expect even while the in-app editor is deferred.
- No way to rename the workspace from the header.
- The repo name isn't shown anywhere in the workspace view.
