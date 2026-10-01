# 01 · App shell and sidebar

## Summary

Harness is a desktop app window with a persistent left sidebar and a main area that shows one of three views: **Homebase**, **Settings**, or a **Workspace**. The sidebar is the global navigation and the always-on status board: it lists every workspace and every agent thread with live status, so the user can see what needs them without opening anything.

## Why it exists

The user supervises several agents at once. The sidebar answers "what needs me right now?" from any screen. It serves the principle **Show what needs you**: status is reduced to working, idle, paused, starting or waiting on approval, and pending approvals are counted in a badge.

## Where it lives

- Element: `<nav class="sidebar" id="sidebar" aria-label="Harness">`.
- Always visible on desktop widths. Collapses to a stacked block above the main area below 760px.
- Hooks: `nav-settings` (settings button). The workspace list is `#side-ws`, the agent list `#side-ag`.

## Anatomy

From top to bottom:

1. **Title bar.** A draggable strip with macOS-style window controls (close, minimise, maximise). The controls are decorative in the prototype (`aria-hidden`). There is no logo and no product name; both were removed deliberately.
2. **Search button.** A full-width field-like button reading **Search** with a `⌘K` key hint. Opens the command palette (see [02](02-command-palette-and-shortcuts.md)).
3. **Homebase** nav item, with a layers icon. Marked `aria-current="page"` while Homebase is showing.
4. **New workspace** nav item, with a plus icon and a `⌘N` key hint. Opens the New workspace dialog (see [06](06-new-workspace.md)).
5. **Repos** section.
   - Section label **Repos** with a count of added repos on the right.
   - One row per repo, in Settings order: a folder icon, the repo name on line 1 and its default branch (monospace) on line 2.
   - A refresh icon button at the right edge (`Pull <branch> for <repo>`) fast-forwards the default branch in the repo's own checkout from origin: `git pull --ff-only` when that branch is checked out there, otherwise `git fetch origin <branch>:<branch>` so the checkout's files aren't touched. The icon spins while it runs; a toast reports the commits pulled or that the branch is up to date, and workspaces' behind counts refresh. Failures (for example a non-fast-forward) toast the git error.
   - Empty state: **No repos yet**.
6. **Workspaces** section.
   - Section label **Workspaces** with a count of all workspaces on the right.
   - One row per workspace, newest first (new workspaces are inserted at the top).
   - Each row: a status dot, the workspace name on line 1, and on line 2 the status word plus agent count, for example `Running · 3 agents`. If any thread in the workspace has approvals waiting, an amber numeric badge sits at the right edge.
   - The row for the open workspace is marked `aria-current="page"` and shown with a stronger background (no coloured side stripe).
   - Empty state: **No workspaces yet**.
7. **Agents** section.
   - Section label **Agents** with `N working` on the right, where N counts threads that are running or provisioning and not paused, across all workspaces.
   - One row per thread across every workspace, in workspace order then thread order.
   - Each row: the engine's two-letter mark (`CC`, `CU`, `CX`, `GM`), line 1 `Lead in Auth session timeout` (role, then "in" and the workspace name in a subtler colour), line 2 the thread's current activity, or `Paused`.
   - Right edge: an amber approval badge if the thread has pending approvals, otherwise the thread's status dot.
   - Empty state: **No agents running**.
8. **Footer.**
   - **Memory meter**: a CPU icon, a horizontal bar and a value like `1.4 GB`. Grouped as "Memory usage" for screen readers.
   - **Settings** icon button (sliders icon). Shows as selected (`aria-current="page"`) while the Settings page is open.

## Status dots and words

The sidebar uses one shared status mapping for workspaces and threads:

| Underlying state | Dot class | Status word (workspace row) |
| --- | --- | --- |
| `provisioning` | `st-prov` | Starting |
| `running`, not paused | `st-running` (green, pulsing ring) | Running |
| `running`, paused | `st-paused` (amber) | Paused |
| `idle` | `st-idle` (grey) | The workspace's activity text, for example `Plan ready` or `Review ready` |

Thread rows always show the activity text (or `Paused`), not the status word.

Note that the workspace **cards** on Homebase use a different, simpler vocabulary ("X agents working", "Idle", "Needs Attention"). See [04](04-homebase-workspace-cards.md). The sidebar is deliberately more granular.

## Behaviour

- **Clicking a workspace row** opens that workspace on its first thread (index 0) with the Thread tab selected, scrolls the conversation to the end and moves focus to the workspace heading.
- **Clicking an agent row** opens the workspace that thread belongs to with that thread's tab selected.
- **Clicking Homebase** returns to Homebase and focuses its heading.
- **Clicking the Settings button** opens the Settings page. Clicking it while Settings is already open does nothing.
- The sidebar re-renders on every state change (approvals, status changes, new threads, teardown), so counts and badges are always current.
- **Memory meter** updates every 4 seconds.

## Views and window title

The main area shows exactly one view at a time. The browser/window title follows it:

| View | Window title |
| --- | --- |
| Homebase | `Cormux · Homebase` |
| Settings | `Cormux · Settings` |
| Workspace | `Cormux · <workspace name>` |

## Responsive behaviour

- **Above 1100px:** sidebar at full width, main area to the right.
- **1100px and below:** sidebar narrows to 216px and window padding drops to 12px.
- **760px and below:** the window becomes a single column. The sidebar stacks above the main area, its scrolling list is capped at 280px tall, the window loses its rounded corners and shadow, and the page itself scrolls.

## Keyboard and accessibility

- Sidebar is a `<nav>` landmark labelled "Harness".
- Each list has a visible label element referenced by `aria-labelledby`.
- Workspace row `aria-label`: `<name>, <status word>, <N agents>[, <N approvals> waiting]`. The visible lines are `aria-hidden` to avoid double reading.
- Agent row `aria-label`: `<role>, <engine name>, in <workspace name>: <activity>[, <N approvals> waiting]`.
- `⌘K` opens search and `⌘N` opens New workspace from anywhere (see [02](02-command-palette-and-shortcuts.md)).

## Data model

Reads from `state.workspaces[]` and each workspace's `extra[]` threads:

- Workspace: `id`, `title`, `status`, `paused`, `activity`, `extra`, `timeline` (for pending approvals).
- Thread: `role`, `engine`, `status`, `paused`, `activity`, `timeline`.
- `state.view` and `state.currentId` decide the current markers.

## Simulated in the prototype

- **Memory meter** is invented: `0.9 GB + 0.24 GB per active thread + up to 0.18 GB random noise`, with the bar filled to `value ÷ 4 GB`. A real build should report the actual resident memory of Harness plus its agent processes, and decide what the bar's maximum means.
- **Window controls** do nothing.

## Known gaps and open questions

- The Agents list can get long with many workspaces. There is no grouping, filtering or collapse. Decide whether it should group by workspace, show only working or waiting threads, or be removed now that thread tabs exist inside each workspace.
- The memory meter has no threshold or warning state. Decide whether high memory should suggest pausing or tearing down workspaces.
- There is no way to reorder, pin or rename workspaces from the sidebar.
- No right-click or overflow menu on rows (for example Teardown, Open in editor).
- The sidebar has no collapsed or icon-only mode for small windows other than the stacked mobile layout.
