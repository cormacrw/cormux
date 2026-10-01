# 02 · Command palette and keyboard shortcuts

## Summary

A `⌘K` command palette lets the user jump to any workspace, run global actions, and control each workspace's app (run, restart, stop) or pull from its base branch, without the mouse. A small set of global shortcuts covers the most frequent actions.

## Why it exists

Harness is keyboard-first. A user supervising several workspaces needs to jump between them and fire common actions faster than navigating the sidebar. The palette also doubles as search.

## Where it lives

- `<dialog class="dialog palette" id="palette" aria-label="Command palette">`.
- Entry points: `⌘K` / `Ctrl+K` anywhere, or the **Search** button at the top of the sidebar.

## Anatomy

1. **Input row**: search icon, a text input with placeholder **Type a command or workspace…**, and an `Esc` key hint.
2. **Result list**: commands grouped under small group headings. Each item has an icon, a label, optional right-aligned meta text, and an optional key hint.
3. **Footer**: `↑ ↓ navigate` and `↵ run`.

## Commands

Commands are rebuilt every time the palette renders, so they always reflect current state. Groups appear in this order:

### Actions (always present)

| Label | Icon | Key hint | Effect |
| --- | --- | --- | --- |
| New workspace | plus | `⌘N` | Opens the New workspace dialog. |
| Go to Homebase | layers | | Shows Homebase. |
| Open settings | sliders | | Shows Settings. |
| Enable reduced motion / Disable reduced motion | sliders | | Toggles the Reduce motion setting. Label flips with the current value. |

### Workspaces (one per workspace)

- Label: `Open <workspace name>`.
- Meta: the same status text as the Homebase card: `2 agents working`, `Idle` or `Needs Attention`.
- Effect: opens the workspace on its first thread.

### App (one or two per workspace that has finished provisioning)

Workspaces still provisioning are excluded.

| App state | Commands | Meta |
| --- | --- | --- |
| Stopped | `Run app in <name>` | The repo's run command, or `No run command` |
| Starting | `Stop app in <name>` | |
| Running | `Restart app in <name>`, then `Stop app in <name>` | Restart shows `localhost:<port>` |

Run and Restart first open the workspace, then perform the action. Stop acts without navigating.

### Git (one per workspace that is behind its base)

- Label: `Pull <N commits> from <base> into <name>`, for example `Pull 2 commits from main into Auth session timeout`.
- Effect: same as the Pull action in the workspace's ⋯ menu (see [17](17-pull-and-rebase.md)). Does not navigate.
- There is no palette command for Rebase.

## Behaviour

- **Opening** clears the input, selects the first item and focuses the input. Any open popover is closed first.
- **Toggle:** pressing `⌘K` while the palette is open closes it.
- **Blocked:** the palette will not open while the New workspace dialog or a confirmation dialog is open.
- **Filtering:** case-insensitive substring match against the label *and* the meta text, so typing `idle` finds idle workspaces and `pnpm` finds Run commands. Group headings are shown only for groups with matches. The selection resets to the first match on every keystroke.
- **No results:** a single line reads `No commands match “<query>”`.
- **Running a command** closes the palette first, then runs the command on the next tick so focus handling in the destination view works.
- **Mouse:** hovering an item selects it; clicking runs it. Clicking the backdrop closes the palette.

## Global keyboard shortcuts

| Shortcut | Where | Effect |
| --- | --- | --- |
| `⌘K` / `Ctrl+K` | Anywhere | Open or close the command palette. |
| `⌘N` / `Ctrl+N` | Anywhere | Open the New workspace dialog. |
| `⌘H` / `Ctrl+H` | Anywhere but a dialog | Go to Homebase. Replaces the macOS Hide shortcut; **Hide Cormux** stays in the app menu without one. |
| `⌘1`–`⌘9` / `Ctrl+1`–`Ctrl+9` | Anywhere but a dialog | Open the Nth workspace in sidebar order. No-op if there isn't one. |
| `⌘G` / `Ctrl+G` | Workspace view, no dialog open | Open the Git tab (stack and diff). |
| `Ctrl+\`` | Workspace view, no dialog open | Toggle between the Output tab and the tab you came from. |
| `Esc` | A popover is open | Close the popover and return focus to its trigger. |
| `←` `→` `Home` `End` | Focus on a workspace tab | Move between thread, Findings and Output tabs. |
| `↵` / `⇧↵` | Composer | Send / new line. |
| `⌘↵` / `Ctrl+↵` | New workspace Initial Prompt, Create PR reason | Submit the form. |

## Keyboard and accessibility

- The input is a `combobox` with `aria-controls="pal-list"`, `aria-expanded="true"` and `aria-activedescendant` pointing at the selected option.
- Items are `role="option"` with `aria-selected`; group headings and the empty line are `role="presentation"`.
- `↑`/`↓` move the selection and keep it scrolled into view inside the list only (never scrolling the page). `↵` runs the selected item.
- `Esc` closes the dialog natively.

## Data model

- Reads `state.workspaces`, `state.settings.reduceMotion`, each workspace's app state (`app.status`, `app.port`), its repo's `run` command, and `behind` / `base`.
- Local state: `palItems` (current filtered list) and `palIdx` (selection).

## Simulated in the prototype

Nothing beyond what the underlying actions simulate.

## Known gaps and open questions

- No commands for: Create PR, Rebase, Teardown, New thread, Switch branch, Submit review, open Findings, open a specific thread, or jump to a Settings section.
- No search across thread content, files, PRs or findings. Decide whether the palette should become a real search.
- No fuzzy matching or ranking; results are in fixed group order.
- No recently used commands.
- `⌘N` is also the browser's "new window" shortcut; in a real desktop shell this is fine, but in a browser build it will conflict.
